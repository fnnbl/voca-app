use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    audio::{AudioBuffer, AudioRecordingState},
    AppState, AppStateManager, AudioDuckingState, RecordingGate,
    RecordingStateChangedPayload, SelectedAudioDevice,
};

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Super";

// Two presses within this window trigger toggle mode (macOS, via on_release tap detection)
const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(400);
// A press released within this window is a "tap" (not a hold)
const TAP_MAX_HOLD: Duration = Duration::from_millis(250);
// Dedup window for on_press — protects against rdev + frontend fallback firing
// the same physical key press twice. 50ms is longer than any realistic gap
// between the two paths and shorter than any realistic human double-tap.
const ON_PRESS_DEDUP: Duration = Duration::from_millis(50);

/// Called by hotkey::start when the shortcut keys are all pressed.
pub fn on_press(app: &AppHandle) {
    // Gate during early onboarding: before the user has reached the Test
    // step (or completed onboarding otherwise), pressing the shortcut must
    // be a no-op. The listener stays alive in the background so we don't
    // have to re-register anything when the gate opens.
    if !*app.state::<RecordingGate>().0.lock().unwrap() {
        eprintln!("[VOCA shortcut] on_press ignored (recording gate locked)");
        return;
    }

    // Dedup against double-firing from rdev + frontend fallback.
    {
        let press_state = app.state::<crate::LastPressTime>();
        let last = press_state.0.lock().unwrap();
        if let Some(t) = *last {
            if Instant::now().duration_since(t) < ON_PRESS_DEDUP {
                eprintln!("[VOCA shortcut] on_press deduped");
                return;
            }
        }
    }

    let app_state = app.state::<AppStateManager>().0.lock().unwrap().clone();
    eprintln!("[VOCA shortcut] on_press in state: {app_state:?}");

    match app_state {
        AppState::Recording => {
            // Always stop on press. Covers toggle-exit and redundant stops.
            stop_recording(app);
        }
        AppState::Idle | AppState::Error => {
            // Error → reset to Idle and start fresh so the shortcut always works.
            if app_state == AppState::Error {
                *app.state::<AppStateManager>().0.lock().unwrap() = AppState::Idle;
                emit_state(app, AppState::Idle);
            }
            let is_double_tap = {
                let tap_state = app.state::<crate::LastTapTime>();
                let last = tap_state.0.lock().unwrap();
                last.map_or(false, |t| Instant::now().duration_since(t) <= DOUBLE_TAP_WINDOW)
            };
            *app.state::<crate::LastPressTime>().0.lock().unwrap() = Some(Instant::now());
            *app.state::<crate::IsToggleSession>().0.lock().unwrap() = is_double_tap;
            start_recording(app);
        }
        _ => {
            eprintln!("[VOCA shortcut] ignoring press in transient state");
        }
    }

    // Stamp LastPressTime so duplicate fires from a second source (frontend
    // fallback) within ON_PRESS_DEDUP are suppressed even when we took the
    // Recording / transient-state branch.
    *app.state::<crate::LastPressTime>().0.lock().unwrap() = Some(Instant::now());
}

/// Called by hotkey::start when any shortcut key is released.
pub fn on_release(app: &AppHandle) {
    let is_toggle = *app.state::<crate::IsToggleSession>().0.lock().unwrap();
    if is_toggle {
        return;
    }

    let app_state = app.state::<AppStateManager>().0.lock().unwrap().clone();
    if app_state != AppState::Recording {
        return;
    }

    let press_state = app.state::<crate::LastPressTime>();
    let held_ms = press_state.0.lock().unwrap()
        .map_or(u64::MAX, |t| Instant::now().duration_since(t).as_millis() as u64);

    if held_ms < TAP_MAX_HOLD.as_millis() as u64 {
        // Quick tap: cancel and set LastTapTime so the next press is detected as double-tap
        *app.state::<crate::LastTapTime>().0.lock().unwrap() = Some(Instant::now());
        cancel_recording(app);
    } else {
        // Normal push-to-talk hold ended
        stop_recording(app);
    }
}

fn start_recording(app: &AppHandle) {
    let device_name = app
        .state::<SelectedAudioDevice>()
        .0
        .lock()
        .unwrap()
        .clone();

    let audio = app.state::<AudioRecordingState>();
    if let Err(e) = crate::audio::start(&audio, device_name.as_deref(), app) {
        crate::errors::emit(app, "MICROPHONE_UNAVAILABLE", &e);
        crate::errors::transition_to_error(app);
        return;
    }
    maybe_duck_audio(app);
    let manager = app.state::<AppStateManager>();
    *manager.0.lock().unwrap() = AppState::Recording;
    emit_state(app, AppState::Recording);
}

/// Public entry point for triggering a normal stop from outside the
/// shortcut module — used by the VAD auto-stop watcher when silence is
/// detected after speech. Does the same work as the internal
/// `stop_recording`, but no-ops if we're not currently recording so a
/// late watcher tick doesn't fight with a manual release.
pub fn stop_recording_external(app: &AppHandle) {
    let state = app.state::<AppStateManager>().0.lock().unwrap().clone();
    if state != AppState::Recording {
        return;
    }
    stop_recording(app);
}

fn stop_recording(app: &AppHandle) {
    let audio = app.state::<AudioRecordingState>();
    match crate::audio::stop(&audio, app) {
        Ok(wav_bytes) => {
            restore_ducked_audio(app);
            *app.state::<AudioBuffer>().0.lock().unwrap() = Some(wav_bytes);
            let manager = app.state::<AppStateManager>();
            *manager.0.lock().unwrap() = AppState::Processing;
            emit_state(app, AppState::Processing);
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::transcription::process(app_clone).await;
            });
        }
        Err(e) => {
            restore_ducked_audio(app);
            crate::errors::emit(app, "RECORDING_FAILED", &e);
            crate::errors::transition_to_error(app);
        }
    }
}

fn cancel_recording(app: &AppHandle) {
    // Called only from quick-tap path. Stop audio immediately but delay the
    // Idle state emit by DOUBLE_TAP_WINDOW so a double-tap → toggle transition
    // keeps the UI pill showing continuously (no Recording→Idle→Recording flash).
    let audio = app.state::<AudioRecordingState>();
    let _ = crate::audio::stop(&audio, app);
    restore_ducked_audio(app);
    let manager = app.state::<AppStateManager>();
    *manager.0.lock().unwrap() = AppState::Idle;

    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(DOUBLE_TAP_WINDOW);
        let current = app_clone.state::<AppStateManager>().0.lock().unwrap().clone();
        if current == AppState::Idle {
            emit_state(&app_clone, AppState::Idle);
        }
    });
}

fn emit_state(app: &AppHandle, state: AppState) {
    crate::update_tray_icon(app, &state);
    let _ = app.emit("recording-state-changed", RecordingStateChangedPayload { state });
}

fn maybe_duck_audio(app: &AppHandle) {
    let settings = crate::storage::load(app).unwrap_or_default();
    let enabled = settings["transcription"]["muteOtherAudio"]
        .as_bool()
        .unwrap_or(true);
    if !enabled {
        return;
    }
    let guard = crate::audio_ducking::mute_others();
    *app.state::<AudioDuckingState>().0.lock().unwrap() = Some(guard);
}

fn restore_ducked_audio(app: &AppHandle) {
    let guard = app.state::<AudioDuckingState>().0.lock().unwrap().take();
    if let Some(g) = guard {
        crate::audio_ducking::restore(g);
    }
}
