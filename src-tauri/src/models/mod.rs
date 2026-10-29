// Live model lists, fetched from each provider's `/models` endpoint with the
// key stored in the keychain. Hardcoded lists in the frontend go stale within
// months (providers retire models constantly), so the UI prefers whatever the
// provider reports for this key and only falls back to its static list when
// this fetch fails.

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::keychain;

/// Days before a provider's announced shutdown date at which the startup
/// check starts warning. The settings page shows the date regardless.
const SHUTDOWN_WARNING_DAYS: i64 = 30;

const ANTHROPIC_MODELS_URL: &str = "https://api.anthropic.com/v1/models?limit=1000";

/// Substrings that mark a model as unusable for text post-processing
/// (embeddings, speech, images, moderation, realtime, guard models, …).
const NON_CHAT_MARKERS: &[&str] = &[
    "embed", "whisper", "transcribe", "tts", "dall-e", "image", "imagen", "veo", "sora",
    "moderation", "realtime", "audio", "live", "search", "guard", "orpheus", "davinci",
    "babbage", "computer-use", "aqa",
];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    /// Announced shutdown day (`YYYY-MM-DD`), if the provider reports one.
    pub shutdown_date: Option<String>,
}

impl ModelInfo {
    fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), shutdown_date: None }
    }
}

pub async fn list_ai_models(provider: &str, custom_endpoint: Option<&str>) -> Result<Vec<ModelInfo>, String> {
    let models = fetch_ai_models(provider, custom_endpoint).await?;
    Ok(finalize(models.into_iter().filter(|m| is_chat_model(&m.id))))
}

pub async fn list_transcription_models(provider: &str, custom_endpoint: Option<&str>) -> Result<Vec<ModelInfo>, String> {
    let models = fetch_transcription_models(provider, custom_endpoint).await?;
    Ok(match provider {
        // Gemini transcribes through regular multimodal generation.
        "gemini" => finalize(models.into_iter().filter(|m| m.id.starts_with("gemini") && is_chat_model(&m.id))),
        // Self-hosted servers name their models freely ("parakeet", …), so
        // a name-based filter would hide valid choices.
        "custom" => finalize(models.into_iter()),
        _ => finalize(models.into_iter().filter(|m| is_transcription_model(&m.id))),
    })
}

/// Unfiltered listing — the startup check compares against this so the
/// name heuristics above can never produce a false "unavailable" warning.
async fn fetch_ai_models(provider: &str, custom_endpoint: Option<&str>) -> Result<Vec<ModelInfo>, String> {
    let key = keychain::get_ai_provider_key(provider)?.unwrap_or_default();
    if key.is_empty() && provider != "ollama" {
        return Err("API_KEY_MISSING: no key stored for this provider".into());
    }
    if provider == "anthropic" {
        fetch_anthropic(&key).await
    } else {
        fetch_openai_compat(&crate::ai::base_url(provider, custom_endpoint), &key).await
    }
}

async fn fetch_transcription_models(provider: &str, custom_endpoint: Option<&str>) -> Result<Vec<ModelInfo>, String> {
    // Deepgram and ElevenLabs have no OpenAI-style model listing; the
    // frontend keeps its static lists for them.
    if !matches!(provider, "openai" | "groq" | "gemini" | "custom") {
        return Err("MODEL_LIST_UNSUPPORTED".into());
    }
    let key = keychain::get_transcription_key(provider)?.unwrap_or_default();
    if key.is_empty() {
        return Err("API_KEY_MISSING: no key stored for this provider".into());
    }
    let base = if provider == "gemini" {
        crate::ai::base_url("gemini", None)
    } else {
        crate::transcription::openai_compat_base_url(provider, custom_endpoint.unwrap_or(""))
    };
    fetch_openai_compat(&base, &key).await
}

async fn fetch_openai_compat(base_url: &str, api_key: &str) -> Result<Vec<ModelInfo>, String> {
    let mut req = crate::http_client().get(format!("{base_url}/models"));
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let json = send(req).await?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    Ok(parse_openai_models(&json, &today))
}

async fn fetch_anthropic(api_key: &str) -> Result<Vec<ModelInfo>, String> {
    let req = crate::http_client()
        .get(ANTHROPIC_MODELS_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01");
    let json = send(req).await?;
    Ok(json["data"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|m| m["id"].as_str().map(ModelInfo::new)).collect())
        .unwrap_or_default())
}

async fn send(req: reqwest::RequestBuilder) -> Result<serde_json::Value, String> {
    let response = req.send().await.map_err(|e| {
        if e.is_connect() || e.is_timeout() {
            format!("NO_INTERNET: {e}")
        } else {
            format!("REQUEST_FAILED: {e}")
        }
    })?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err("API_KEY_INVALID: 401 Unauthorized".into());
    }
    if !status.is_success() {
        return Err(format!("REQUEST_FAILED: HTTP {status}"));
    }
    response.json().await.map_err(|e| format!("REQUEST_FAILED: {e}"))
}

/// Extract models from an OpenAI-shaped `{"data": [{"id": …}]}` listing.
/// Drops models whose `shutdown_date` has passed and strips Gemini's
/// `models/` prefix, which its chat endpoint does not expect.
fn parse_openai_models(json: &serde_json::Value, today: &str) -> Vec<ModelInfo> {
    let Some(data) = json["data"].as_array() else {
        return Vec::new();
    };
    data.iter()
        .filter_map(|m| {
            let id = m["id"].as_str()?;
            let shutdown_date = shutdown_day(&m["shutdown_date"]);
            if shutdown_date.as_deref().is_some_and(|d| d <= today) {
                return None;
            }
            Some(ModelInfo {
                id: id.strip_prefix("models/").unwrap_or(id).to_owned(),
                shutdown_date,
            })
        })
        .collect()
}

/// `shutdown_date` may arrive as a unix timestamp or an ISO date string;
/// anything else (null, missing) means no shutdown is announced.
fn shutdown_day(value: &serde_json::Value) -> Option<String> {
    if let Some(ts) = value.as_i64() {
        return chrono::DateTime::from_timestamp(ts, 0).map(|d| d.format("%Y-%m-%d").to_string());
    }
    value.as_str().and_then(|s| s.get(..10)).map(str::to_owned)
}

fn is_chat_model(id: &str) -> bool {
    let id = id.to_lowercase();
    !NON_CHAT_MARKERS.iter().any(|m| id.contains(m))
}

/// Models usable with a single uploaded file on `/audio/transcriptions`.
/// Diarize variants reject the `prompt` field we send; live/realtime
/// variants are streaming-only.
fn is_transcription_model(id: &str) -> bool {
    let id = id.to_lowercase();
    (id.contains("whisper") || id.contains("transcribe"))
        && !id.contains("diarize")
        && !id.contains("live")
        && !id.contains("realtime")
}

fn finalize(models: impl Iterator<Item = ModelInfo>) -> Vec<ModelInfo> {
    let mut models: Vec<ModelInfo> = models.collect();
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    models
}

// ── Startup check ──────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum ModelIssue {
    Unavailable,
    /// Still served, but shuts down on the given day.
    Retiring(String),
}

/// Compare the configured model against a live listing. `warn_from` is the
/// latest shutdown day that is close enough to be worth a warning.
fn evaluate_model(model: &str, available: &[ModelInfo], warn_from: &str) -> Option<ModelIssue> {
    match available.iter().find(|m| m.id == model) {
        None => Some(ModelIssue::Unavailable),
        Some(ModelInfo { shutdown_date: Some(day), .. }) if day.as_str() <= warn_from => {
            Some(ModelIssue::Retiring(day.clone()))
        }
        Some(_) => None,
    }
}

/// Check the configured transcription and enhancement models once after
/// launch and notify the user before a dictation fails on them. Only talks
/// to providers the user already uses, with the keys already stored.
/// Listing failures (offline, no key) are logged and otherwise ignored.
pub async fn run_startup_check(app: AppHandle) {
    let settings = crate::storage::load(&app).unwrap_or_default();
    let warn_from = (chrono::Utc::now() + chrono::Duration::days(SHUTDOWN_WARNING_DAYS))
        .format("%Y-%m-%d")
        .to_string();

    let transcription = &settings["transcription"];
    if transcription["mode"].as_str().unwrap_or("cloud") != "local" {
        let provider = transcription["cloudProvider"].as_str().unwrap_or("openai");
        let configured = transcription["cloudModel"].as_str().unwrap_or("");
        let model = if configured.is_empty() {
            crate::transcription::default_cloud_model(provider)
        } else {
            configured
        };
        let endpoint = transcription["cloudCustomEndpoint"].as_str();
        match fetch_transcription_models(provider, endpoint).await {
            Ok(list) => notify_issue(&app, "transcription", model, evaluate_model(model, &list, &warn_from)),
            Err(e) => log::info!("model check: transcription list unavailable: {e}"),
        }
    }

    let ai = &settings["aiEnhancement"];
    let model = ai["model"].as_str().unwrap_or("");
    if ai["enabled"].as_bool().unwrap_or(false) && !model.is_empty() {
        let provider = ai["provider"].as_str().unwrap_or("openai");
        let endpoint = ai["customEndpoint"].as_str().filter(|s| !s.is_empty());
        match fetch_ai_models(provider, endpoint).await {
            Ok(list) => notify_issue(&app, "AI enhancement", model, evaluate_model(model, &list, &warn_from)),
            Err(e) => log::info!("model check: AI list unavailable: {e}"),
        }
    }
}

fn notify_issue(app: &AppHandle, area: &str, model: &str, issue: Option<ModelIssue>) {
    let body = match issue {
        None => return,
        Some(ModelIssue::Unavailable) => format!(
            "Your {area} model \u{201c}{model}\u{201d} is no longer available. Choose another one in Settings."
        ),
        Some(ModelIssue::Retiring(day)) => format!(
            "Your {area} model \u{201c}{model}\u{201d} will be shut down on {day}. Choose another one in Settings."
        ),
    };
    log::warn!("model check: {body}");
    let _ = app.notification().builder().title("VOCA").body(body).show();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ids(models: &[ModelInfo]) -> Vec<&str> {
        models.iter().map(|m| m.id.as_str()).collect()
    }

    #[test]
    fn parses_ids_and_strips_gemini_prefix() {
        let listing = json!({"data": [{"id": "gpt-x"}, {"id": "models/gemini-3.8-flash"}]});
        assert_eq!(ids(&parse_openai_models(&listing, "2026-10-29")), vec!["gpt-x", "gemini-3.8-flash"]);
    }

    #[test]
    fn drops_models_past_shutdown_date() {
        let listing = json!({"data": [
            {"id": "gone", "shutdown_date": "2026-10-23"},
            {"id": "today", "shutdown_date": "2026-10-29"},
            {"id": "later", "shutdown_date": "2027-02-26"},
            {"id": "gone-ts", "shutdown_date": 1_700_000_000},
            {"id": "none", "shutdown_date": null},
        ]});
        let parsed = parse_openai_models(&listing, "2026-10-29");
        assert_eq!(ids(&parsed), vec!["later", "none"]);
        assert_eq!(parsed[0].shutdown_date.as_deref(), Some("2027-02-26"));
        assert_eq!(parsed[1].shutdown_date, None);
    }

    #[test]
    fn evaluate_flags_missing_and_soon_retiring_models() {
        let list = vec![
            ModelInfo { id: "soon".into(), shutdown_date: Some("2026-11-15".into()) },
            ModelInfo { id: "far".into(), shutdown_date: Some("2027-02-26".into()) },
            ModelInfo::new("fine"),
        ];
        let warn_from = "2026-11-28";
        assert_eq!(evaluate_model("gone", &list, warn_from), Some(ModelIssue::Unavailable));
        assert_eq!(evaluate_model("soon", &list, warn_from), Some(ModelIssue::Retiring("2026-11-15".into())));
        assert_eq!(evaluate_model("far", &list, warn_from), None);
        assert_eq!(evaluate_model("fine", &list, warn_from), None);
    }

    #[test]
    fn missing_data_array_yields_empty_list() {
        assert!(parse_openai_models(&json!({"error": "nope"}), "2026-10-29").is_empty());
    }

    #[test]
    fn chat_filter_excludes_non_text_models() {
        for id in ["text-embedding-3-small", "whisper-1", "gpt-transcribe", "tts-1", "gpt-image-1",
                   "omni-moderation-latest", "gpt-realtime", "meta-llama/llama-prompt-guard-2-22m",
                   "canopylabs/orpheus-v1-english", "gemini-3.8-live"] {
            assert!(!is_chat_model(id), "{id} should be excluded");
        }
        for id in ["gpt-6-luna", "claude-haiku-5-5", "openai/gpt-oss-120b", "llama3.2:latest",
                   "gemini-3.5-flash-lite", "mistral-small-latest"] {
            assert!(is_chat_model(id), "{id} should be kept");
        }
    }

    #[test]
    fn transcription_filter_keeps_file_transcription_models_only() {
        for id in ["whisper-1", "gpt-transcribe", "gpt-4o-mini-transcribe", "whisper-large-v3-turbo"] {
            assert!(is_transcription_model(id), "{id} should be kept");
        }
        for id in ["gpt-4o-transcribe-diarize", "gpt-live-transcribe", "gpt-realtime-whisper", "gpt-6-luna"] {
            assert!(!is_transcription_model(id), "{id} should be excluded");
        }
    }

    #[test]
    fn finalize_sorts_and_dedups() {
        let out = finalize(vec![ModelInfo::new("b"), ModelInfo::new("a"), ModelInfo::new("b")].into_iter());
        assert_eq!(ids(&out), vec!["a", "b"]);
    }
}
