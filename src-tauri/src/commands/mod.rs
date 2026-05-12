use std::path::PathBuf;

use tauri::{Emitter, Manager, State, Theme};
use tauri_plugin_autostart::ManagerExt;

use crate::{
    keychain::{self, KeyType},
    storage,
    storage::AiPrompt,
    AppState, AppStateManager, HotkeyStateWrapper, ModelDownloadProgressPayload, ModelStatus,
    RecordingStateChangedPayload, SelectedAudioDevice,
};

#[tauri::command]
pub fn get_app_state(state: State<AppStateManager>) -> AppState {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_app_state(
    app: tauri::AppHandle,
    state: State<AppStateManager>,
    new_state: AppState,
) -> Result<(), String> {
    let mut current = state.0.lock().map_err(|e| e.to_string())?;
    *current = new_state.clone();
    drop(current);

    app.emit("recording-state-changed", RecordingStateChangedPayload { state: new_state })
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn register_shortcut(
    app: tauri::AppHandle,
    new_shortcut: String,
) -> Result<(), String> {
    let hotkey = app.state::<HotkeyStateWrapper>();
    hotkey.0.set_shortcut(&new_shortcut);
    Ok(())
}

/// Frontend-fallback entry points for the shortcut. Some platforms miss the
/// rdev low-level hook when VOCA's own window has focus (WebView captures
/// events first). The frontend listens for DOM keyboard events that match
/// the current shortcut and invokes these — the backend dedups against a
/// near-simultaneous rdev fire via LastPressTime.
#[tauri::command]
pub fn trigger_shortcut_press(app: tauri::AppHandle) {
    crate::shortcut::on_press(&app);
}

#[tauri::command]
pub fn trigger_shortcut_release(app: tauri::AppHandle) {
    crate::shortcut::on_release(&app);
}

#[tauri::command]
pub fn save_api_key(key_type: KeyType, value: String) -> Result<(), String> {
    keychain::save(key_type, &value)
}

#[tauri::command]
pub fn get_api_key(key_type: KeyType) -> Result<Option<String>, String> {
    keychain::get(key_type)
}

#[tauri::command]
pub fn delete_api_key(key_type: KeyType) -> Result<(), String> {
    keychain::delete(key_type)
}

#[tauri::command]
pub fn get_settings(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    storage::load(&app)
}

#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    settings: serde_json::Value,
) -> Result<(), String> {
    // Intercept the onboardingCompleted transition from false → true. This
    // covers every completion path (Done step, skip button, closing the
    // window mid-flow) in one spot so frontend callers don't have to
    // remember to unlock the gate and show the pill themselves.
    let prev = storage::load(&app).ok();
    let was_done = prev
        .as_ref()
        .and_then(|s| s["general"]["onboardingCompleted"].as_bool())
        .unwrap_or(false);
    let is_done = settings["general"]["onboardingCompleted"]
        .as_bool()
        .unwrap_or(false);

    storage::save(&app, &settings)?;

    if !was_done && is_done {
        // Completion just happened — unlock recording and reveal pill if it
        // was hidden. We don't animate here; this path is for users who
        // skipped past the Test step. The animated reveal only fires via
        // the `pill-animate-reveal` event from the Test step mount.
        *app.state::<crate::RecordingGate>().0.lock().unwrap() = true;
        if let Some(pill) = app.get_webview_window("status-bar") {
            let _ = pill.show();
            // Re-assert click-through and topmost — see show_pill for why.
            let _ = pill.set_ignore_cursor_events(true);
            let _ = pill.set_always_on_top(true);
        }
    }

    Ok(())
}

#[tauri::command]
pub fn unlock_recording(app: tauri::AppHandle) -> Result<(), String> {
    *app.state::<crate::RecordingGate>().0.lock().unwrap() = true;
    Ok(())
}

#[tauri::command]
pub fn show_pill(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(pill) = app.get_webview_window("status-bar") {
        pill.show().map_err(|e| e.to_string())?;
        // Re-assert click-through after show(). Windows drops the
        // ignore-cursor flag across a hide/show cycle, which would
        // leave the pill blocking clicks on apps underneath.
        let _ = pill.set_ignore_cursor_events(true);
        // Same story for topmost — hide/show drops HWND_TOPMOST and the
        // pill ends up behind any normal window (only visible on the
        // bare desktop). Re-asserting keeps it floating above everything.
        let _ = pill.set_always_on_top(true);
    }
    Ok(())
}

/// Dismiss the update-toast speech bubble. Called when the user clicks
/// "Später" or after the auto-dismiss timeout.
#[tauri::command]
pub fn dismiss_update_toast(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(toast) = app.get_webview_window("update-toast") {
        toast.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// User clicked "Update?" on the speech bubble. Hide the toast, surface
/// the main window, and tell the frontend to switch to the About page so
/// the user can see the full update flow there.
#[tauri::command]
pub fn accept_update_toast(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(toast) = app.get_webview_window("update-toast") {
        let _ = toast.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
        main.emit("updater://navigate-to-about", ())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_model_status(app: tauri::AppHandle, size: String) -> Result<ModelStatus, String> {
    let path = model_path(&app, &size)?;
    let (downloaded, size_bytes) = if path.exists() {
        let meta = std::fs::metadata(&path).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
        (true, meta.len())
    } else {
        (false, 0)
    };
    Ok(ModelStatus { downloaded, size_bytes })
}

#[tauri::command]
pub async fn download_model(app: tauri::AppHandle, size: String) -> Result<(), String> {
    let valid = ["tiny", "base", "small", "medium"];
    if !valid.contains(&size.as_str()) {
        return Err(format!("Invalid model size: {size}"));
    }

    *app.state::<crate::DownloadCancelFlag>().0.lock().unwrap() = false;

    let url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{size}.bin"
    );
    let dest = model_path(&app, &size)?;
    let tmp = dest.with_extension("bin.tmp");

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    }

    let client = reqwest::Client::new();
    let mut response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("DOWNLOAD_FAILED: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("DOWNLOAD_FAILED: HTTP {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0);
    let mut downloaded_bytes: u64 = 0;

    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("STORAGE_ERROR: {e}"))?;

    use tokio::io::AsyncWriteExt;
    loop {
        if *app.state::<crate::DownloadCancelFlag>().0.lock().unwrap() {
            drop(file);
            let _ = std::fs::remove_file(&tmp);
            return Err("DOWNLOAD_CANCELLED".into());
        }
        match response.chunk().await.map_err(|e| format!("DOWNLOAD_FAILED: {e}"))? {
            None => break,
            Some(chunk) => {
                file.write_all(&chunk)
                    .await
                    .map_err(|e| format!("STORAGE_ERROR: {e}"))?;
                downloaded_bytes += chunk.len() as u64;
                let _ = app.emit(
                    "model-download-progress",
                    ModelDownloadProgressPayload {
                        size: size.clone(),
                        downloaded_bytes,
                        total_bytes: total,
                    },
                );
            }
        }
    }

    file.flush().await.map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    drop(file);

    std::fs::rename(&tmp, &dest).map_err(|e| format!("STORAGE_ERROR: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn delete_model(app: tauri::AppHandle, size: String) -> Result<(), String> {
    let valid = ["tiny", "base", "small", "medium"];
    if !valid.contains(&size.as_str()) {
        return Err(format!("Invalid model size: {size}"));
    }
    let dest = model_path(&app, &size)?;
    if dest.exists() {
        std::fs::remove_file(&dest).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_model_download(app: tauri::AppHandle) -> Result<(), String> {
    *app.state::<crate::DownloadCancelFlag>().0.lock().unwrap() = true;
    Ok(())
}

#[derive(Clone, serde::Serialize)]
pub struct CustomModelInfo {
    pub filename: String,
    pub size_bytes: u64,
}

#[derive(Clone, serde::Serialize)]
pub struct ImportModelResult {
    pub filename: String,
    pub size_bytes: u64,
}

/// Whisper.cpp model files start with magic bytes. The legacy GGML
/// format uses ASCII `ggml`; the newer GGUF format uses ASCII `GGUF`.
/// Anything else is rejected before we let `whisper-rs` try to load it
/// (which would otherwise abort the whole process on malformed input).
fn validate_whisper_model_magic(path: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    let mut header = [0u8; 4];
    file.read_exact(&mut header)
        .map_err(|_| "INVALID_MODEL: file too short to be a Whisper model".to_string())?;
    if &header == b"ggml" || &header == b"GGUF" {
        Ok(())
    } else {
        Err("INVALID_MODEL: not a whisper.cpp compatible model (expected ggml or GGUF header)".to_string())
    }
}

/// Accepts only safe basenames with a `.bin` extension (case-insensitive
/// on the extension, original case preserved on the stem). Anything that
/// looks like a path component, hidden file, or contains characters that
/// are invalid on Windows is rejected.
fn sanitise_model_filename(raw: &str) -> Result<String, String> {
    if !raw.to_ascii_lowercase().ends_with(".bin") {
        return Err("INVALID_MODEL: file must have a .bin extension".to_string());
    }
    let stem = &raw[..raw.len() - 4];
    if stem.is_empty()
        || stem.starts_with('.')
        || stem.contains('/')
        || stem.contains('\\')
        || stem.contains("..")
        || stem
            .chars()
            .any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\0'))
    {
        return Err("INVALID_MODEL: filename contains invalid characters".to_string());
    }
    Ok(format!("{stem}.bin"))
}

pub fn custom_models_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("models").join("custom"))
        .map_err(|e| format!("STORAGE_ERROR: {e}"))
}

pub fn custom_model_path(app: &tauri::AppHandle, filename: &str) -> Result<PathBuf, String> {
    let safe = sanitise_model_filename(filename)?;
    Ok(custom_models_dir(app)?.join(safe))
}

#[tauri::command]
pub fn list_custom_models(app: tauri::AppHandle) -> Result<Vec<CustomModelInfo>, String> {
    let dir = custom_models_dir(&app)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("STORAGE_ERROR: {e}"))? {
        let entry = entry.map_err(|e| format!("STORAGE_ERROR: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.to_ascii_lowercase().ends_with(".bin") => n.to_owned(),
            _ => continue,
        };
        let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
        out.push(CustomModelInfo { filename, size_bytes });
    }
    out.sort_by(|a, b| a.filename.to_ascii_lowercase().cmp(&b.filename.to_ascii_lowercase()));
    Ok(out)
}

#[tauri::command]
pub async fn import_custom_model(
    app: tauri::AppHandle,
    source_path: String,
    overwrite: bool,
) -> Result<ImportModelResult, String> {
    let source = std::path::PathBuf::from(&source_path);
    let raw_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "INVALID_MODEL: source path has no filename".to_string())?
        .to_owned();
    let filename = sanitise_model_filename(&raw_name)?;

    validate_whisper_model_magic(&source)?;

    let dest = custom_model_path(&app, &filename)?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    }

    if dest.exists() && !overwrite {
        return Err("MODEL_ALREADY_EXISTS".to_string());
    }

    // Stage via a sibling .tmp then atomic-rename so a partial copy
    // (interrupted multi-GB transfer) never appears in the model list.
    let tmp = dest.with_extension("bin.tmp");
    tokio::fs::copy(&source, &tmp)
        .await
        .map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    tokio::fs::rename(&tmp, &dest)
        .await
        .map_err(|e| format!("STORAGE_ERROR: {e}"))?;

    let size_bytes = dest.metadata().map(|m| m.len()).unwrap_or(0);
    Ok(ImportModelResult { filename, size_bytes })
}

#[tauri::command]
pub fn delete_custom_model(app: tauri::AppHandle, filename: String) -> Result<(), String> {
    let dest = custom_model_path(&app, &filename)?;
    if dest.exists() {
        std::fs::remove_file(&dest).map_err(|e| format!("STORAGE_ERROR: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_transcription_key(provider: String, value: String) -> Result<(), String> {
    keychain::save_transcription_key(&provider, &value)
}

#[tauri::command]
pub fn get_transcription_key(provider: String) -> Result<Option<String>, String> {
    keychain::get_transcription_key(&provider)
}

#[tauri::command]
pub fn delete_transcription_key(provider: String) -> Result<(), String> {
    keychain::delete_transcription_key(&provider)
}

#[tauri::command]
pub fn save_ai_provider_key(provider: String, value: String) -> Result<(), String> {
    keychain::save_ai_provider_key(&provider, &value)
}

#[tauri::command]
pub fn get_ai_provider_key(provider: String) -> Result<Option<String>, String> {
    keychain::get_ai_provider_key(&provider)
}

#[tauri::command]
pub fn delete_ai_provider_key(provider: String) -> Result<(), String> {
    keychain::delete_ai_provider_key(&provider)
}

#[tauri::command]
pub fn get_prompts(app: tauri::AppHandle) -> Result<Vec<AiPrompt>, String> {
    storage::load_prompts(&app)
}

#[tauri::command]
pub fn save_prompts(app: tauri::AppHandle, prompts: Vec<AiPrompt>) -> Result<(), String> {
    storage::save_prompts(&app, &prompts)
}

#[tauri::command]
pub fn get_dictionary(app: tauri::AppHandle) -> Result<Vec<crate::storage::DictionaryEntry>, String> {
    crate::storage::load_dictionary(&app)
}

#[tauri::command]
pub fn save_dictionary(app: tauri::AppHandle, entries: Vec<crate::storage::DictionaryEntry>) -> Result<(), String> {
    crate::storage::save_dictionary(&app, &entries)
}

#[tauri::command]
pub fn seed_dictionary_with_use_cases(
    app: tauri::AppHandle,
    use_cases: Vec<String>,
) -> Result<(), String> {
    let existing = crate::storage::load_dictionary(&app).unwrap_or_default();

    // Case-insensitive dedupe against existing entries — we never duplicate a
    // word the user already has, and we never overwrite their choices.
    let mut existing_keys: std::collections::HashSet<String> =
        existing.iter().map(|e| e.word.to_lowercase()).collect();

    let refs: Vec<&str> = use_cases.iter().map(|s| s.as_str()).collect();
    let seed_words = crate::storage::dictionary_seeds::seeds_for(&refs);

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let mut merged = existing;
    for (index, word) in seed_words.into_iter().enumerate() {
        let key = word.to_lowercase();
        if existing_keys.insert(key) {
            merged.push(crate::storage::DictionaryEntry {
                id: format!("seed-{nanos}-{index}"),
                word,
            });
        }
    }

    crate::storage::save_dictionary(&app, &merged)
}

#[tauri::command]
pub fn get_snippets(app: tauri::AppHandle) -> Result<Vec<crate::storage::Snippet>, String> {
    crate::storage::load_snippets(&app)
}

#[tauri::command]
pub fn save_snippets(app: tauri::AppHandle, snippets: Vec<crate::storage::Snippet>) -> Result<(), String> {
    crate::storage::save_snippets(&app, &snippets)
}

#[tauri::command]
pub fn get_fillers(app: tauri::AppHandle) -> Result<crate::storage::FillersFile, String> {
    crate::storage::load_fillers(&app)
}

#[tauri::command]
pub fn save_fillers(
    app: tauri::AppHandle,
    fillers: crate::storage::FillersFile,
) -> Result<(), String> {
    crate::storage::save_fillers(&app, &fillers)
}

#[tauri::command]
pub fn get_filler_suggestions(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let history = crate::storage::load_history(&app)?;
    let fillers = crate::storage::load_fillers(&app)?;
    Ok(crate::fillers::compute_suggestions(&history, &fillers))
}

#[tauri::command]
pub fn reject_filler_suggestion(app: tauri::AppHandle, word: String) -> Result<(), String> {
    let mut fillers = crate::storage::load_fillers(&app)?;
    let lower = word.to_lowercase();
    if !fillers.rejected.iter().any(|w| w.to_lowercase() == lower) {
        fillers.rejected.push(word);
    }
    crate::storage::save_fillers(&app, &fillers)
}

#[tauri::command]
pub fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_history(app: tauri::AppHandle) -> Result<Vec<crate::storage::HistoryEntry>, String> {
    crate::storage::load_history(&app)
}

#[tauri::command]
pub fn get_stats(app: tauri::AppHandle) -> Result<crate::stats::StatsSummary, String> {
    let history = crate::storage::load_history(&app)?;
    let now_ms = chrono::Local::now().timestamp_millis();
    Ok(crate::stats::aggregate(&history, now_ms))
}

#[tauri::command]
pub fn clear_history(app: tauri::AppHandle) -> Result<(), String> {
    crate::storage::clear_history(&app)
}

#[tauri::command]
pub fn set_window_theme(app: tauri::AppHandle, theme: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let t = match theme.as_str() {
            "dark"  => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            _       => None,
        };
        window.set_theme(t).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_audio_devices() -> Vec<String> {
    crate::audio::list_input_devices()
}

#[tauri::command]
pub fn set_audio_device(state: State<SelectedAudioDevice>, name: Option<String>) {
    *state.0.lock().unwrap() = name;
}

pub fn model_path(app: &tauri::AppHandle, size: &str) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("models").join(format!("ggml-{size}.bin")))
        .map_err(|e| format!("STORAGE_ERROR: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sanitise_accepts_normal_basename() {
        assert_eq!(sanitise_model_filename("my-model.bin").unwrap(), "my-model.bin");
        assert_eq!(sanitise_model_filename("ggml-large-v3.bin").unwrap(), "ggml-large-v3.bin");
        assert_eq!(sanitise_model_filename("German_Fine_Tune.BIN").unwrap(), "German_Fine_Tune.bin");
    }

    #[test]
    fn sanitise_rejects_path_components() {
        assert!(sanitise_model_filename("../escape.bin").is_err());
        assert!(sanitise_model_filename("dir/model.bin").is_err());
        assert!(sanitise_model_filename("dir\\model.bin").is_err());
    }

    #[test]
    fn sanitise_rejects_non_bin() {
        assert!(sanitise_model_filename("model.txt").is_err());
        assert!(sanitise_model_filename("model").is_err());
        assert!(sanitise_model_filename(".bin").is_err());
    }

    #[test]
    fn sanitise_rejects_windows_reserved_chars() {
        assert!(sanitise_model_filename("a:b.bin").is_err());
        assert!(sanitise_model_filename("a*b.bin").is_err());
        assert!(sanitise_model_filename("a?b.bin").is_err());
        assert!(sanitise_model_filename("a|b.bin").is_err());
    }

    fn write_temp(bytes: &[u8]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(bytes).unwrap();
        f
    }

    #[test]
    fn magic_accepts_ggml_header() {
        let f = write_temp(b"ggml\0\0\0\0extra bytes");
        assert!(validate_whisper_model_magic(f.path()).is_ok());
    }

    #[test]
    fn magic_accepts_gguf_header() {
        let f = write_temp(b"GGUF\0\0\0\0extra bytes");
        assert!(validate_whisper_model_magic(f.path()).is_ok());
    }

    #[test]
    fn magic_rejects_garbage() {
        let f = write_temp(b"not a model file at all");
        let err = validate_whisper_model_magic(f.path()).unwrap_err();
        assert!(err.contains("INVALID_MODEL"));
    }

    #[test]
    fn magic_rejects_too_short_file() {
        let f = write_temp(b"gg");
        let err = validate_whisper_model_magic(f.path()).unwrap_err();
        assert!(err.contains("INVALID_MODEL"));
    }
}
