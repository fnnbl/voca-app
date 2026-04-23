use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::{audio::AudioBuffer, keychain, AppState, AppStateManager};

#[derive(Clone, serde::Serialize)]
pub struct TranscriptionResultPayload {
    pub text: String,
}

pub async fn process(app: AppHandle) {
    let wav_bytes = {
        let buf = app.state::<AudioBuffer>();
        let x = buf.0.lock().unwrap().take();
        x
    };

    let wav_bytes = match wav_bytes {
        Some(b) => b,
        None => {
            transition_to_idle(&app);
            return;
        }
    };

    let settings = crate::storage::load(&app).unwrap_or_default();

    // Transcription language is an independent setting from UI language.
    // "auto" (the default) lets the STT provider detect the language itself,
    // which avoids forcing a wrong hint when the speaker mixes languages.
    let language = settings["transcription"]["language"]
        .as_str()
        .unwrap_or("auto")
        .to_owned();

    let mode = settings["transcription"]["mode"]
        .as_str()
        .unwrap_or("cloud")
        .to_owned();

    let dict_prompt = build_dict_prompt(&app);
    let duration_secs = wav_duration_secs(&wav_bytes);

    // Silence / very-short recordings: skip transcription entirely. Whisper
    // (and some cloud models) tend to hallucinate canned phrases like
    // "Vielen Dank fürs Zuschauen" on empty audio.
    if duration_secs < 0.5 || is_audio_silent(&wav_bytes) {
        let _ = app.emit(
            "transcription-result",
            TranscriptionResultPayload { text: String::new() },
        );
        transition_to_idle(&app);
        return;
    }

    let result = if mode == "local" {
        transcribe_local(&app, &settings, wav_bytes, &language, dict_prompt.as_deref()).await
    } else {
        transcribe_cloud(&app, &settings, wav_bytes, &language, dict_prompt.as_deref()).await
    };

    let raw_text = match result {
        Ok(text) if text.trim().is_empty() => {
            let _ = app.emit(
                "transcription-result",
                TranscriptionResultPayload { text: String::new() },
            );
            transition_to_idle(&app);
            return;
        }
        Ok(text) if is_likely_hallucination(&text, duration_secs) => {
            eprintln!("[VOCA transcription] filtered hallucination: {text:?}");
            let _ = app.emit(
                "transcription-result",
                TranscriptionResultPayload { text: String::new() },
            );
            transition_to_idle(&app);
            return;
        }
        Ok(text) => text,
        Err(e) => {
            let (code, message) = classify_error(&e);
            crate::errors::emit(&app, code, &message);
            crate::errors::transition_to_error(&app);
            return;
        }
    };

    let expanded_text = apply_snippets(&app, raw_text);
    let pre_enhance = expanded_text.clone();
    let final_text = maybe_enhance(&app, &settings, expanded_text).await;

    let was_enhanced = final_text != pre_enhance;
    let word_count = final_text.split_whitespace().count() as u32;
    let provider = if mode == "local" {
        "local".to_owned()
    } else {
        settings["transcription"]["cloudProvider"]
            .as_str()
            .unwrap_or("cloud")
            .to_owned()
    };

    let paste_text = format!("{} ", final_text.trim());

    let history_tracking = settings["privacy"]["historyTracking"]
        .as_bool()
        .unwrap_or(true);
    let target_app_tracking = settings["privacy"]["targetAppTracking"]
        .as_bool()
        .unwrap_or(false);

    // Capture must happen while the user's target app is still frontmost —
    // i.e. before we simulate Ctrl+V. Skip entirely when either toggle is off.
    let target_app = if history_tracking && target_app_tracking {
        crate::target_app::capture()
    } else {
        None
    };

    transition_to_inserting(&app);
    if let Err(e) = crate::clipboard::paste(&paste_text) {
        crate::errors::emit(&app, "PASTE_FAILED", &e);
        crate::errors::transition_to_error(&app);
        return;
    }
    transition_to_idle(&app);

    if history_tracking {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let _ = crate::storage::append_history_entry(&app, crate::storage::HistoryEntry {
            id: timestamp_ms.to_string(),
            timestamp_ms,
            text: final_text.trim().to_owned(),
            enhanced: was_enhanced,
            duration_secs,
            word_count,
            provider,
            target_app,
        });
    }

    let _ = app.emit("transcription-result", TranscriptionResultPayload { text: final_text.trim().to_owned() });
}

// ── Local transcription ────────────────────────────────────────────────────────

async fn transcribe_local(
    app: &AppHandle,
    settings: &serde_json::Value,
    wav_bytes: Vec<u8>,
    language: &str,
    initial_prompt: Option<&str>,
) -> Result<String, String> {
    let model_size = settings["transcription"]["localModelSize"]
        .as_str()
        .unwrap_or("base")
        .to_owned();
    let model_path = crate::commands::model_path(app, &model_size)?;

    if !model_path.exists() {
        crate::errors::emit(
            app,
            "LOCAL_MODEL_MISSING",
            &format!("Model '{model_size}' not downloaded yet"),
        );
        crate::errors::transition_to_error(app);
        return Err(String::new());
    }

    let model_path_str = model_path.to_string_lossy().to_string();
    let lang = language.to_owned();
    let prompt = initial_prompt.map(|s| s.to_owned());

    let cache_arc = Arc::clone(&app.state::<crate::WhisperModelCache>().0);

    tokio::task::spawn_blocking(move || {
        let mut lock = cache_arc.lock().unwrap();
        let needs_reload = lock.as_ref().map(|(p, _)| p.as_str()) != Some(model_path_str.as_str());
        if needs_reload {
            let ctx = crate::local_transcription::load_context(&model_path_str)?;
            *lock = Some((model_path_str, ctx));
        }
        let (_, ctx) = lock.as_ref().unwrap();
        crate::local_transcription::transcribe_with_context(ctx, &wav_bytes, &lang, prompt.as_deref())
    })
    .await
    .unwrap_or_else(|e| Err(format!("LOCAL_MODEL_ERROR: {e}")))
}

// ── Cloud transcription ────────────────────────────────────────────────────────

async fn transcribe_cloud(
    app: &AppHandle,
    settings: &serde_json::Value,
    wav_bytes: Vec<u8>,
    language: &str,
    initial_prompt: Option<&str>,
) -> Result<String, String> {
    let provider = settings["transcription"]["cloudProvider"]
        .as_str()
        .unwrap_or("openai")
        .to_owned();
    let model = settings["transcription"]["cloudModel"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let custom_endpoint = settings["transcription"]["cloudCustomEndpoint"]
        .as_str()
        .unwrap_or("")
        .to_owned();

    let api_key = match keychain::get_transcription_key(&provider) {
        Ok(Some(k)) => k,
        Ok(None) => {
            crate::errors::emit(app, "API_KEY_MISSING", "No API key configured for transcription");
            crate::errors::transition_to_error(app);
            return Err(String::new());
        }
        Err(e) => {
            crate::errors::emit(app, "KEYCHAIN_ERROR", &e);
            crate::errors::transition_to_error(app);
            return Err(String::new());
        }
    };

    call_cloud(&provider, wav_bytes, &api_key, language, &model, &custom_endpoint, initial_prompt).await
}

async fn call_cloud(
    provider: &str,
    wav_bytes: Vec<u8>,
    api_key: &str,
    language: &str,
    model: &str,
    custom_endpoint: &str,
    initial_prompt: Option<&str>,
) -> Result<String, String> {
    match provider {
        "deepgram" => {
            let m = if model.is_empty() { "nova-3" } else { model };
            call_deepgram(wav_bytes, api_key, language, m, initial_prompt).await
        }
        "elevenlabs" => {
            let m = if model.is_empty() { "scribe_v1" } else { model };
            call_elevenlabs(wav_bytes, api_key, language, m).await
        }
        "gemini" => {
            let m = if model.is_empty() { "gemini-2.0-flash" } else { model };
            call_gemini(wav_bytes, api_key, language, m, initial_prompt).await
        }
        _ => {
            let base = match provider {
                "groq" => "https://api.groq.com/openai/v1".to_string(),
                "custom" => {
                    let ep = custom_endpoint.trim_end_matches('/');
                    if ep.ends_with("/v1") { ep.to_string() } else { format!("{ep}/v1") }
                }
                _ => "https://api.openai.com/v1".to_string(),
            };
            let m = if model.is_empty() {
                if provider == "groq" { "whisper-large-v3" } else { "whisper-1" }
            } else {
                model
            };
            call_openai_compat(wav_bytes, api_key, language, m, &base, initial_prompt).await
        }
    }
}

async fn call_openai_compat(
    wav_bytes: Vec<u8>,
    api_key: &str,
    language: &str,
    model: &str,
    base_url: &str,
    initial_prompt: Option<&str>,
) -> Result<String, String> {
    let part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("REQUEST_FAILED: {e}"))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", model.to_owned());
    if !is_auto_language(language) {
        form = form.text("language", language.to_owned());
    }
    if let Some(prompt) = initial_prompt {
        form = form.text("prompt", prompt.to_owned());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{base_url}/audio/transcriptions"))
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
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
        let body = response.text().await.unwrap_or_default();
        return Err(format!("REQUEST_FAILED: HTTP {status} – {body}"));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("REQUEST_FAILED: {e}"))?;

    json["text"]
        .as_str()
        .map(|s| s.to_owned())
        .ok_or_else(|| "REQUEST_FAILED: missing 'text' field in response".into())
}

async fn call_deepgram(
    wav_bytes: Vec<u8>,
    api_key: &str,
    language: &str,
    model: &str,
    initial_prompt: Option<&str>,
) -> Result<String, String> {
    let keywords = initial_prompt
        .map(|p| {
            p.split(',')
                .map(|w| format!("&keywords={}", w.trim()))
                .collect::<String>()
        })
        .unwrap_or_default();
    let language_param = if is_auto_language(language) {
        "detect_language=true".to_owned()
    } else {
        format!("language={language}")
    };
    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "https://api.deepgram.com/v1/listen?{language_param}&model={model}&smart_format=true{keywords}"
        ))
        .header("Authorization", format!("Token {api_key}"))
        .header("Content-Type", "audio/wav")
        .body(wav_bytes)
        .send()
        .await
        .map_err(|e| {
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
        let body = response.text().await.unwrap_or_default();
        return Err(format!("REQUEST_FAILED: HTTP {status} – {body}"));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("REQUEST_FAILED: {e}"))?;

    json["results"]["channels"][0]["alternatives"][0]["transcript"]
        .as_str()
        .map(|s| s.to_owned())
        .ok_or_else(|| "REQUEST_FAILED: unexpected Deepgram response shape".into())
}

async fn call_elevenlabs(
    wav_bytes: Vec<u8>,
    api_key: &str,
    language: &str,
    model: &str,
) -> Result<String, String> {
    let part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("REQUEST_FAILED: {e}"))?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model_id", model.to_owned());

    if !is_auto_language(language) {
        form = form.text("language_code", language.to_owned());
    }

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.elevenlabs.io/v1/speech-to-text")
        .header("xi-api-key", api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
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
        let body = response.text().await.unwrap_or_default();
        return Err(format!("REQUEST_FAILED: HTTP {status} – {body}"));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("REQUEST_FAILED: {e}"))?;

    json["text"]
        .as_str()
        .map(|s| s.to_owned())
        .ok_or_else(|| "REQUEST_FAILED: unexpected ElevenLabs response shape".into())
}

async fn call_gemini(
    wav_bytes: Vec<u8>,
    api_key: &str,
    language: &str,
    model: &str,
    initial_prompt: Option<&str>,
) -> Result<String, String> {
    use base64::Engine;
    let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);

    let vocab_hint = initial_prompt
        .filter(|p| !p.is_empty())
        .map(|p| format!(" Pay attention to these terms: {p}."))
        .unwrap_or_default();

    let prompt = if is_auto_language(language) {
        format!("Transcribe the following audio. Return only the transcribed text, nothing else.{vocab_hint}")
    } else {
        format!("Transcribe the following audio in {language}. Return only the transcribed text, nothing else.{vocab_hint}")
    };

    let body = serde_json::json!({
        "contents": [{
            "parts": [
                {"text": prompt},
                {"inline_data": {"mime_type": "audio/wav", "data": audio_b64}}
            ]
        }]
    });

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
        ))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                format!("NO_INTERNET: {e}")
            } else {
                format!("REQUEST_FAILED: {e}")
            }
        })?;

    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status.as_u16() == 400 {
        let body = response.text().await.unwrap_or_default();
        if body.contains("API_KEY") {
            return Err("API_KEY_INVALID: invalid Gemini API key".into());
        }
        return Err(format!("REQUEST_FAILED: HTTP {status} – {body}"));
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("REQUEST_FAILED: HTTP {status} – {body}"));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("REQUEST_FAILED: {e}"))?;

    json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.trim().to_owned())
        .ok_or_else(|| "REQUEST_FAILED: unexpected Gemini response shape".into())
}

// ── AI enhancement ─────────────────────────────────────────────────────────────

async fn maybe_enhance(
    app: &AppHandle,
    settings: &serde_json::Value,
    text: String,
) -> String {
    let enabled = settings["aiEnhancement"]["enabled"].as_bool().unwrap_or(false);
    if !enabled {
        return text;
    }

    if settings["aiEnhancement"]["skipShortTranscriptions"]
        .as_bool()
        .unwrap_or(true)
    {
        let min_words = settings["aiEnhancement"]["minWords"].as_u64().unwrap_or(3) as usize;
        if text.split_whitespace().count() < min_words {
            return text;
        }
    }

    let provider = settings["aiEnhancement"]["provider"]
        .as_str()
        .unwrap_or("openai")
        .to_owned();
    let model = settings["aiEnhancement"]["model"]
        .as_str()
        .unwrap_or("gpt-4o")
        .to_owned();
    let active_prompt_id = settings["aiEnhancement"]["activePromptId"]
        .as_str()
        .unwrap_or("default")
        .to_owned();
    let custom_endpoint = settings["aiEnhancement"]["customEndpoint"]
        .as_str()
        .map(|s| s.to_owned());

    let api_key = match keychain::get_ai_provider_key(&provider) {
        Ok(Some(k)) => k,
        _ if provider == "ollama" => String::new(),
        _ => return text,
    };

    let prompt_text = match crate::storage::load_prompts(app) {
        Ok(prompts) => prompts
            .into_iter()
            .find(|p| p.id == active_prompt_id)
            .map(|p| p.prompt)
            .unwrap_or_default(),
        Err(_) => return text,
    };

    if prompt_text.is_empty() {
        return text;
    }

    match crate::ai::enhance(&text, &prompt_text, &provider, &model, &api_key, custom_endpoint.as_deref()).await {
        Ok(enhanced) if !enhanced.trim().is_empty() => enhanced,
        Ok(_) => text,
        Err(e) => {
            let _ = app
                .notification()
                .builder()
                .title("VOCA")
                .body("AI enhancement failed – original text was used.")
                .show();
            log::warn!("AI enhancement failed: {e}");
            text
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

pub(crate) fn is_auto_language(language: &str) -> bool {
    language.is_empty() || language.eq_ignore_ascii_case("auto")
}

fn build_dict_prompt(app: &AppHandle) -> Option<String> {
    let entries = crate::storage::load_dictionary(app).ok()?;
    dict_entries_to_prompt(&entries)
}

pub(crate) fn dict_entries_to_prompt(entries: &[crate::storage::DictionaryEntry]) -> Option<String> {
    if entries.is_empty() {
        return None;
    }
    Some(entries.iter().map(|e| e.word.as_str()).collect::<Vec<_>>().join(", "))
}

fn apply_snippets(app: &AppHandle, text: String) -> String {
    let snippets = match crate::storage::load_snippets(app) {
        Ok(s) => s,
        Err(_) => return text,
    };
    apply_snippets_to_text(text, &snippets)
}

pub(crate) fn apply_snippets_to_text(text: String, snippets: &[crate::storage::Snippet]) -> String {
    let enabled: Vec<_> = snippets.iter().filter(|s| s.enabled).collect();
    if enabled.is_empty() {
        return text;
    }
    let mut result = text;
    for snippet in &enabled {
        if snippet.trigger.is_empty() {
            continue;
        }
        let pattern = format!(r"(?i)\b{}\b", regex_escape(&snippet.trigger));
        if let Ok(re) = regex::Regex::new(&pattern) {
            result = re.replace_all(&result, snippet.output.as_str()).into_owned();
        }
    }
    result
}

pub(crate) fn regex_escape(s: &str) -> String {
    let special = r"\.+*?()|[]{}^$#&-~";
    s.chars()
        .flat_map(|c| {
            if special.contains(c) { vec!['\\', c] } else { vec![c] }
        })
        .collect()
}

fn wav_duration_secs(wav: &[u8]) -> f32 {
    if wav.len() < 44 { return 0.0; }
    let sample_rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]) as f32;
    let num_channels = u16::from_le_bytes([wav[22], wav[23]]) as f32;
    let bits_per_sample = u16::from_le_bytes([wav[34], wav[35]]) as f32;
    let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as f32;
    if sample_rate == 0.0 || num_channels == 0.0 || bits_per_sample == 0.0 { return 0.0; }
    data_size / (sample_rate * num_channels * (bits_per_sample / 8.0))
}

/// Rough RMS check on 16-bit PCM WAV data. Returns true if the average
/// amplitude is below a threshold indicating the recording is essentially silent.
fn is_audio_silent(wav: &[u8]) -> bool {
    if wav.len() < 44 { return true; }
    let data = &wav[44..];
    if data.is_empty() { return true; }
    let mut sum_sq: f64 = 0.0;
    let mut count: u64 = 0;
    for chunk in data.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f64 / i16::MAX as f64;
        sum_sq += sample * sample;
        count += 1;
    }
    if count == 0 { return true; }
    let rms = (sum_sq / count as f64).sqrt();
    // Empirically: room noise ~0.002-0.005, whispered speech ~0.02, normal speech ~0.1.
    // Keep threshold conservative to avoid cutting quiet speakers; the
    // hallucination filter is a secondary safety net.
    rms < 0.005
}

/// Detects Whisper "silence hallucinations": canned phrases the model emits
/// when given essentially-silent audio. Only applied for short recordings,
/// where the risk of a false positive on real speech is low.
fn is_likely_hallucination(text: &str, duration_secs: f32) -> bool {
    if duration_secs > 4.0 {
        return false;
    }
    let normalized = text
        .trim()
        .trim_end_matches(|c: char| ".,!?".contains(c))
        .to_lowercase();
    const HALLUCINATIONS: &[&str] = &[
        // German
        "vielen dank",
        "vielen dank fürs zuschauen",
        "vielen dank fürs zusehen",
        "danke fürs zuschauen",
        "danke fürs zusehen",
        "untertitel im auftrag des zdf",
        "untertitel von stephanie geiges",
        "untertitelung aufgrund der amara.org-community",
        "untertitelung des zdf, 2020",
        "untertitelung des zdf, 2017",
        "tschüss",
        // English
        "thank you",
        "thanks for watching",
        "thanks",
        "bye",
        "you",
        // Symbols/minimal
        "",
        ".",
        "...",
        "-",
    ];
    HALLUCINATIONS.iter().any(|&h| normalized == h)
}

fn classify_error(e: &str) -> (&'static str, String) {
    if e.starts_with("API_KEY_INVALID") {
        ("API_KEY_INVALID", e.to_owned())
    } else if e.starts_with("NO_INTERNET") {
        ("NO_INTERNET", e.to_owned())
    } else if e.is_empty() {
        // already handled upstream
        ("TRANSCRIPTION_FAILED", e.to_owned())
    } else {
        ("TRANSCRIPTION_FAILED", e.to_owned())
    }
}

fn transition_to_idle(app: &AppHandle) {
    let manager = app.state::<AppStateManager>();
    *manager.0.lock().unwrap() = AppState::Idle;
    crate::update_tray_icon(app, &AppState::Idle);
    let _ = app.emit(
        "recording-state-changed",
        crate::RecordingStateChangedPayload { state: AppState::Idle },
    );
}

fn transition_to_inserting(app: &AppHandle) {
    let manager = app.state::<AppStateManager>();
    *manager.0.lock().unwrap() = AppState::Inserting;
    crate::update_tray_icon(app, &AppState::Inserting);
    let _ = app.emit(
        "recording-state-changed",
        crate::RecordingStateChangedPayload { state: AppState::Inserting },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{DictionaryEntry, Snippet};

    fn make_snippet(trigger: &str, output: &str, enabled: bool) -> Snippet {
        Snippet {
            id: "test".into(),
            name: "".into(),
            trigger: trigger.into(),
            output: output.into(),
            enabled,
            created_at: "".into(),
        }
    }

    fn make_entry(word: &str) -> DictionaryEntry {
        DictionaryEntry { id: "test".into(), word: word.into() }
    }

    // ── is_auto_language ──────────────────────────────────────────────────────

    #[test]
    fn is_auto_language_matches_empty_string() {
        assert!(is_auto_language(""));
    }

    #[test]
    fn is_auto_language_matches_auto_case_insensitively() {
        assert!(is_auto_language("auto"));
        assert!(is_auto_language("Auto"));
        assert!(is_auto_language("AUTO"));
    }

    #[test]
    fn is_auto_language_rejects_explicit_codes() {
        assert!(!is_auto_language("de"));
        assert!(!is_auto_language("en"));
        assert!(!is_auto_language("es"));
    }

    // ── apply_snippets_to_text ────────────────────────────────────────────────

    #[test]
    fn snippet_replaces_whole_word() {
        let snippets = vec![make_snippet("sig", "Best regards, John", true)];
        assert_eq!(
            apply_snippets_to_text("please add sig here".into(), &snippets),
            "please add Best regards, John here"
        );
    }

    #[test]
    fn snippet_case_insensitive() {
        let snippets = vec![make_snippet("sig", "Best regards, John", true)];
        assert_eq!(
            apply_snippets_to_text("SIG at end".into(), &snippets),
            "Best regards, John at end"
        );
    }

    #[test]
    fn snippet_no_partial_match() {
        let snippets = vec![make_snippet("sig", "REPLACED", true)];
        assert_eq!(
            apply_snippets_to_text("my signature here".into(), &snippets),
            "my signature here"
        );
    }

    #[test]
    fn snippet_disabled_not_applied() {
        let snippets = vec![make_snippet("sig", "REPLACED", false)];
        assert_eq!(
            apply_snippets_to_text("please add sig here".into(), &snippets),
            "please add sig here"
        );
    }

    #[test]
    fn snippet_empty_trigger_skipped() {
        let snippets = vec![make_snippet("", "REPLACED", true)];
        assert_eq!(
            apply_snippets_to_text("hello world".into(), &snippets),
            "hello world"
        );
    }

    #[test]
    fn snippet_multiple_occurrences() {
        let snippets = vec![make_snippet("thx", "Thank you", true)];
        assert_eq!(
            apply_snippets_to_text("thx and thx again".into(), &snippets),
            "Thank you and Thank you again"
        );
    }

    #[test]
    fn snippet_empty_list_returns_unchanged() {
        assert_eq!(
            apply_snippets_to_text("hello world".into(), &[]),
            "hello world"
        );
    }

    // ── regex_escape ──────────────────────────────────────────────────────────

    #[test]
    fn regex_escape_plain_text() {
        assert_eq!(regex_escape("hello"), "hello");
    }

    #[test]
    fn regex_escape_dot() {
        assert_eq!(regex_escape("e.g"), r"e\.g");
    }

    #[test]
    fn regex_escape_parens() {
        assert_eq!(regex_escape("(test)"), r"\(test\)");
    }

    // ── dict_entries_to_prompt ────────────────────────────────────────────────

    #[test]
    fn dict_prompt_empty_returns_none() {
        assert_eq!(dict_entries_to_prompt(&[]), None);
    }

    #[test]
    fn dict_prompt_single_word() {
        assert_eq!(dict_entries_to_prompt(&[make_entry("VOCA")]), Some("VOCA".into()));
    }

    #[test]
    fn dict_prompt_multiple_words_comma_separated() {
        let entries = vec![make_entry("VOCA"), make_entry("Tauri"), make_entry("Whisper")];
        assert_eq!(dict_entries_to_prompt(&entries), Some("VOCA, Tauri, Whisper".into()));
    }

    // ── classify_error ────────────────────────────────────────────────────────

    #[test]
    fn classify_api_key_invalid() {
        let (code, _) = classify_error("API_KEY_INVALID: 401");
        assert_eq!(code, "API_KEY_INVALID");
    }

    #[test]
    fn classify_no_internet() {
        let (code, _) = classify_error("NO_INTERNET: connection refused");
        assert_eq!(code, "NO_INTERNET");
    }

    #[test]
    fn classify_empty_error_is_handled() {
        let (code, _) = classify_error("");
        assert_eq!(code, "TRANSCRIPTION_FAILED");
    }

    #[test]
    fn classify_unknown_error() {
        let (code, _) = classify_error("some random error");
        assert_eq!(code, "TRANSCRIPTION_FAILED");
    }
}
