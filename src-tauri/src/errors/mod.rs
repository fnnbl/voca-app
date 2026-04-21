use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::{AppState, AppStateManager, ErrorPayload, RecordingStateChangedPayload};

pub fn emit(app: &AppHandle, code: &str, message: &str) {
    let _ = app.emit(
        "error-occurred",
        ErrorPayload {
            code: code.into(),
            message: message.into(),
        },
    );

    let body = notification_body(code);
    let _ = app
        .notification()
        .builder()
        .title("VOCA")
        .body(body)
        .show();
}

pub fn transition_to_error(app: &AppHandle) {
    let manager = app.state::<AppStateManager>();
    *manager.0.lock().unwrap() = AppState::Error;
    crate::update_tray_icon(app, &AppState::Error);
    let _ = app.emit(
        "recording-state-changed",
        RecordingStateChangedPayload {
            state: AppState::Error,
        },
    );
}

pub(crate) fn notification_body(code: &str) -> &'static str {
    match code {
        "API_KEY_MISSING" => "No Whisper API key set. Open Settings to add one.",
        "API_KEY_INVALID" => "Whisper API key is invalid. Check Settings.",
        "NO_INTERNET" => "No internet connection.",
        "MICROPHONE_UNAVAILABLE" => "Microphone unavailable. Check system permissions.",
        "RECORDING_FAILED" => "Recording failed unexpectedly.",
        "SHORTCUT_CONFLICT" => "Could not register shortcut. Choose a different one in Settings.",
        "CLIPBOARD_ERROR" | "PASTE_FAILED" => "Could not insert text into the active window.",
        "LOCAL_MODEL_MISSING" => "Whisper model not downloaded yet. Open Settings to download it.",
        "LOCAL_MODEL_ERROR" => "Local transcription failed.",
        "AI_ENHANCEMENT_FAILED" => "AI enhancement failed – original text was used.",
        "AI_KEY_INVALID" => "AI enhancement API key is invalid.",
        _ => "An error occurred.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes_return_specific_messages() {
        assert!(notification_body("API_KEY_MISSING").contains("API key"));
        assert!(notification_body("API_KEY_INVALID").contains("invalid"));
        assert!(notification_body("NO_INTERNET").contains("internet"));
        assert!(notification_body("MICROPHONE_UNAVAILABLE").contains("Microphone"));
        assert!(notification_body("RECORDING_FAILED").contains("Recording"));
        assert!(notification_body("SHORTCUT_CONFLICT").contains("shortcut"));
        assert!(notification_body("LOCAL_MODEL_MISSING").contains("model"));
        assert!(notification_body("LOCAL_MODEL_ERROR").contains("transcription"));
        assert!(notification_body("AI_ENHANCEMENT_FAILED").contains("enhancement"));
        assert!(notification_body("AI_KEY_INVALID").contains("API key"));
    }

    #[test]
    fn clipboard_error_and_paste_failed_same_message() {
        assert_eq!(notification_body("CLIPBOARD_ERROR"), notification_body("PASTE_FAILED"));
    }

    #[test]
    fn unknown_code_returns_generic_message() {
        assert_eq!(notification_body("SOME_UNKNOWN_CODE"), "An error occurred.");
        assert_eq!(notification_body(""), "An error occurred.");
    }

    #[test]
    fn all_known_codes_return_non_empty_strings() {
        let codes = [
            "API_KEY_MISSING", "API_KEY_INVALID", "NO_INTERNET",
            "MICROPHONE_UNAVAILABLE", "RECORDING_FAILED", "SHORTCUT_CONFLICT",
            "CLIPBOARD_ERROR", "PASTE_FAILED", "LOCAL_MODEL_MISSING",
            "LOCAL_MODEL_ERROR", "AI_ENHANCEMENT_FAILED", "AI_KEY_INVALID",
        ];
        for code in codes {
            assert!(!notification_body(code).is_empty(), "empty message for {code}");
        }
    }
}
