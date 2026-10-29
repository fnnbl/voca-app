// Live model lists, fetched from each provider's `/models` endpoint with the
// key stored in the keychain. Hardcoded lists in the frontend go stale within
// months (providers retire models constantly), so the UI prefers whatever the
// provider reports for this key and only falls back to its static list when
// this fetch fails.

use crate::keychain;

const ANTHROPIC_MODELS_URL: &str = "https://api.anthropic.com/v1/models?limit=1000";

/// Substrings that mark a model as unusable for text post-processing
/// (embeddings, speech, images, moderation, realtime, guard models, …).
const NON_CHAT_MARKERS: &[&str] = &[
    "embed", "whisper", "transcribe", "tts", "dall-e", "image", "imagen", "veo", "sora",
    "moderation", "realtime", "audio", "live", "search", "guard", "orpheus", "davinci",
    "babbage", "computer-use", "aqa",
];

pub async fn list_ai_models(provider: &str, custom_endpoint: Option<&str>) -> Result<Vec<String>, String> {
    let key = keychain::get_ai_provider_key(provider)?.unwrap_or_default();
    if key.is_empty() && provider != "ollama" {
        return Err("API_KEY_MISSING: no key stored for this provider".into());
    }

    let ids = if provider == "anthropic" {
        fetch_anthropic(&key).await?
    } else {
        fetch_openai_compat(&crate::ai::base_url(provider, custom_endpoint), &key).await?
    };

    Ok(finalize(ids.into_iter().filter(|id| is_chat_model(id))))
}

pub async fn list_transcription_models(provider: &str, custom_endpoint: Option<&str>) -> Result<Vec<String>, String> {
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
    let ids = fetch_openai_compat(&base, &key).await?;

    Ok(if provider == "gemini" {
        // Gemini transcribes through regular multimodal generation.
        finalize(ids.into_iter().filter(|id| id.starts_with("gemini") && is_chat_model(id)))
    } else {
        finalize(ids.into_iter().filter(|id| is_transcription_model(id)))
    })
}

async fn fetch_openai_compat(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let mut req = crate::http_client().get(format!("{base_url}/models"));
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let json = send(req).await?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    Ok(parse_openai_models(&json, &today))
}

async fn fetch_anthropic(api_key: &str) -> Result<Vec<String>, String> {
    let req = crate::http_client()
        .get(ANTHROPIC_MODELS_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01");
    let json = send(req).await?;
    Ok(json["data"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|m| m["id"].as_str().map(str::to_owned)).collect())
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

/// Extract model IDs from an OpenAI-shaped `{"data": [{"id": …}]}` listing.
/// Drops models whose `shutdown_date` has passed and strips Gemini's
/// `models/` prefix, which its chat endpoint does not expect.
fn parse_openai_models(json: &serde_json::Value, today: &str) -> Vec<String> {
    let Some(data) = json["data"].as_array() else {
        return Vec::new();
    };
    data.iter()
        .filter(|m| !is_shut_down(&m["shutdown_date"], today))
        .filter_map(|m| m["id"].as_str())
        .map(|id| id.strip_prefix("models/").unwrap_or(id).to_owned())
        .collect()
}

/// `shutdown_date` may arrive as a unix timestamp or an ISO date string;
/// anything else (null, missing) counts as "no shutdown announced".
fn is_shut_down(value: &serde_json::Value, today: &str) -> bool {
    if let Some(ts) = value.as_i64() {
        return chrono::DateTime::from_timestamp(ts, 0)
            .map(|d| d.format("%Y-%m-%d").to_string().as_str() <= today)
            .unwrap_or(false);
    }
    value
        .as_str()
        .and_then(|s| s.get(..10))
        .map(|date| date <= today)
        .unwrap_or(false)
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

fn finalize(ids: impl Iterator<Item = String>) -> Vec<String> {
    let mut ids: Vec<String> = ids.collect();
    ids.sort();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_ids_and_strips_gemini_prefix() {
        let listing = json!({"data": [{"id": "gpt-x"}, {"id": "models/gemini-3.8-flash"}]});
        assert_eq!(parse_openai_models(&listing, "2026-10-29"), vec!["gpt-x", "gemini-3.8-flash"]);
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
        assert_eq!(parse_openai_models(&listing, "2026-10-29"), vec!["later", "none"]);
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
        let out = finalize(vec!["b".to_string(), "a".to_string(), "b".to_string()].into_iter());
        assert_eq!(out, vec!["a", "b"]);
    }
}
