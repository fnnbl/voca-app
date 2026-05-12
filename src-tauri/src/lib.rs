pub mod ai;
pub mod audio;
pub mod audio_ducking;
pub mod clipboard;
pub mod commands;
pub mod errors;
pub mod fillers;
pub mod hotkey;
pub mod keychain;
pub mod local_transcription;
pub mod shortcut;
pub mod stats;
pub mod storage;
pub mod target_app;
pub mod transcription;
pub mod vad;

use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};

use audio::{AudioBuffer, AudioRecordingState};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppState {
    Idle,
    Recording,
    Processing,
    Inserting,
    Error,
}

pub struct AppStateManager(pub Mutex<AppState>);

pub struct WhisperModelCache(pub Arc<Mutex<Option<(String, whisper_rs::WhisperContext)>>>);

pub struct LastPressTime(pub Mutex<Option<std::time::Instant>>);
pub struct LastTapTime(pub Mutex<Option<std::time::Instant>>);
pub struct IsToggleSession(pub Mutex<bool>);
pub struct SelectedAudioDevice(pub Mutex<Option<String>>);
pub struct AudioDuckingState(pub Mutex<Option<audio_ducking::DuckingGuard>>);

/// Blocks recording + pill visibility during early onboarding steps.
/// `false` = gated (ignore shortcut presses, pill window hidden);
/// `true` = normal operation. Set at startup from `general.onboardingCompleted`,
/// flipped to `true` by the frontend when the onboarding Test step mounts
/// (or when onboarding completes by any path).
pub struct RecordingGate(pub Mutex<bool>);
pub struct DownloadCancelFlag(pub Arc<Mutex<bool>>);
pub struct HotkeyStateWrapper(pub std::sync::Arc<hotkey::HotkeyState>);

#[derive(Clone, serde::Serialize)]
pub struct RecordingStateChangedPayload {
    pub state: AppState,
}

#[derive(Clone, serde::Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

#[derive(Clone, serde::Serialize)]
pub struct AudioLevelPayload {
    pub level: f32,
}

#[derive(Clone, serde::Serialize)]
pub struct ModelDownloadProgressPayload {
    pub size: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Clone, serde::Serialize)]
pub struct ModelStatus {
    pub downloaded: bool,
    pub size_bytes: u64,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppStateManager(Mutex::new(AppState::Idle)))
        .manage(AudioRecordingState::new())
        .manage(AudioBuffer(Mutex::new(None)))
        .manage(WhisperModelCache(Arc::new(Mutex::new(None))))
        .manage(LastPressTime(Mutex::new(None)))
        .manage(LastTapTime(Mutex::new(None)))
        .manage(IsToggleSession(Mutex::new(false)))
        .manage(SelectedAudioDevice(Mutex::new(None)))
        .manage(AudioDuckingState(Mutex::new(None)))
        .manage(RecordingGate(Mutex::new(true))) // default; setup() reconciles from settings
        .manage(DownloadCancelFlag(Arc::new(Mutex::new(false))))
        .manage(HotkeyStateWrapper(Arc::new(hotkey::HotkeyState::new())))
        .setup(|app| {
            setup_tray(app)?;

            {
                let handle = app.handle();
                let settings = crate::storage::load(handle).unwrap_or_default();

                // Restore selected audio device from settings
                if let Some(device) = settings["general"]["audioInputDevice"].as_str() {
                    *app.state::<SelectedAudioDevice>().0.lock().unwrap() = Some(device.to_owned());
                }

                let key = settings["shortcuts"]["key"]
                    .as_str()
                    .or_else(|| settings["shortcuts"]["pushToTalk"].as_str())
                    .unwrap_or(shortcut::DEFAULT_SHORTCUT)
                    .to_owned();

                let hotkey_state = app.state::<HotkeyStateWrapper>().0.clone();
                hotkey_state.set_shortcut(&key);
                hotkey::start(hotkey_state, handle.clone());
            }

            // Reconcile the RecordingGate + pill visibility against persisted
            // onboarding state. On first-run installs (onboardingCompleted =
            // false) we gate everything until the user reaches the Test step;
            // on every subsequent launch the pill is normal from the first
            // frame.
            let onboarding_done = {
                let handle = app.handle();
                let settings = crate::storage::load(handle).unwrap_or_default();
                settings["general"]["onboardingCompleted"]
                    .as_bool()
                    .unwrap_or(false)
            };
            *app.state::<RecordingGate>().0.lock().unwrap() = onboarding_done;

            if let Some(status_bar) = app.get_webview_window("status-bar") {
                // The pill window starts hidden (tauri.conf.json
                // `visible: false`) so the OS-chosen initial position is
                // never visible. We pick the active monitor (the one the
                // cursor is on), position deterministically, then show.
                let monitors = status_bar.available_monitors().unwrap_or_default();
                let cursor_monitor = app
                    .cursor_position()
                    .ok()
                    .and_then(|cursor| monitor_for_cursor(&monitors, cursor))
                    .and_then(|idx| monitors.get(idx).cloned());
                let initial_monitor = cursor_monitor
                    .or_else(|| status_bar.primary_monitor().ok().flatten())
                    .or_else(|| status_bar.current_monitor().ok().flatten());
                let initial_monitor_name = initial_monitor
                    .as_ref()
                    .and_then(|m| m.name().cloned());
                if let Some(m) = &initial_monitor {
                    position_status_bar(&status_bar, m);
                }
                // Click-through so the pill never blocks apps underneath.
                // The frontend also calls this on mount, but that Ignore
                // state is not persistent on Windows — it gets dropped on
                // every hide/show cycle. Setting it here at window-setup
                // time closes the race where the pill is already visible
                // before the webview JS has finished mounting.
                let _ = status_bar.set_ignore_cursor_events(true);
                // Topmost is similarly non-persistent on Windows: the
                // tauri.conf.json `alwaysOnTop` flag sets the initial
                // HWND_TOPMOST style, but hide/show cycles can drop it,
                // leaving the pill behind any normal window. Re-asserting
                // here matches the click-through belt-and-suspenders.
                let _ = status_bar.set_always_on_top(true);
                if onboarding_done {
                    let _ = status_bar.show();
                }

                // Watch for active-monitor changes (cursor crossing
                // monitor boundaries). The pill follows monitors, not
                // pixel-level cursor movement.
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    watch_active_monitor(app_handle, initial_monitor_name).await;
                });
            }

            if let Some(toast) = app.get_webview_window("update-toast") {
                // The toast stays hidden in tauri.conf.json (`visible: false`).
                // Position is set lazily in `show_update_toast` based on the
                // pill's current monitor. Always-on-top is asserted here since
                // the config flag is non-persistent across hide/show on
                // Windows, same caveat as the pill.
                let _ = toast.set_always_on_top(true);
            }

            // Optional auto-update check on launch. Default OFF — opt-in via
            // Settings → Privacy → "Check for updates automatically". Skipped
            // during onboarding because the feature surface isn't reachable
            // yet anyway. Delayed by 5s so the boot path stays responsive.
            let auto_check_enabled = {
                let handle = app.handle();
                let settings = crate::storage::load(handle).unwrap_or_default();
                settings["privacy"]["autoCheckUpdates"]
                    .as_bool()
                    .unwrap_or(false)
            };
            if onboarding_done && auto_check_enabled {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    check_for_updates_silently(app_handle).await;
                });
            }

            if let Some(main) = app.get_webview_window("main") {
                // Use small icon for title bar — avoids Windows scaling artefacts
                if let Ok(icon) = tauri::image::Image::from_bytes(
                    include_bytes!("../icons/32x32.png")
                ) {
                    let _ = main.set_icon(icon);
                }

                // Hide on close instead of destroying the window, so the tray
                // icon can bring it back. Without this the webview is torn down
                // and `get_webview_window("main")` returns None.
                let main_clone = main.clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        let _ = main_clone.hide();
                        api.prevent_close();
                    }
                });

                let _ = main.show();
                let _ = main.set_focus();
            }

            #[cfg(debug_assertions)]
            app.get_webview_window("main").map(|w| w.open_devtools());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::set_app_state,
            commands::register_shortcut,
            commands::trigger_shortcut_press,
            commands::trigger_shortcut_release,
            commands::save_api_key,
            commands::get_api_key,
            commands::delete_api_key,
            commands::get_settings,
            commands::save_settings,
            commands::get_model_status,
            commands::download_model,
            commands::delete_model,
            commands::cancel_model_download,
            commands::list_custom_models,
            commands::import_custom_model,
            commands::delete_custom_model,
            commands::get_prompts,
            commands::save_prompts,
            commands::save_transcription_key,
            commands::get_transcription_key,
            commands::delete_transcription_key,
            commands::save_ai_provider_key,
            commands::get_ai_provider_key,
            commands::delete_ai_provider_key,
            commands::get_dictionary,
            commands::save_dictionary,
            commands::seed_dictionary_with_use_cases,
            commands::get_snippets,
            commands::save_snippets,
            commands::get_fillers,
            commands::save_fillers,
            commands::get_filler_suggestions,
            commands::reject_filler_suggestion,
            commands::get_autostart,
            commands::set_autostart,
            commands::get_history,
            commands::clear_history,
            commands::get_stats,
            commands::set_window_theme,
            commands::list_audio_devices,
            commands::set_audio_device,
            commands::unlock_recording,
            commands::show_pill,
            commands::dismiss_update_toast,
            commands::accept_update_toast,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn position_status_bar(window: &tauri::WebviewWindow, monitor: &tauri::Monitor) {
    let scale = monitor.scale_factor();
    let size = monitor.size();
    let origin = monitor.position();
    let win_w = (380.0 * scale) as i32;
    let win_h = (72.0 * scale) as i32;
    let x = origin.x + (size.width as i32 - win_w) / 2;
    let y = origin.y + size.height as i32 - win_h - (56.0 * scale) as i32;
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

fn monitor_for_cursor(
    monitors: &[tauri::Monitor],
    cursor: tauri::PhysicalPosition<f64>,
) -> Option<usize> {
    monitors.iter().position(|m| {
        let mp = m.position();
        let ms = m.size();
        let x = cursor.x as i32;
        let y = cursor.y as i32;
        x >= mp.x
            && x < mp.x + ms.width as i32
            && y >= mp.y
            && y < mp.y + ms.height as i32
    })
}

async fn watch_active_monitor(app: tauri::AppHandle, initial_monitor_name: Option<String>) {
    use std::time::Duration;
    let mut last_name = initial_monitor_name;
    loop {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let Some(status_bar) = app.get_webview_window("status-bar") else {
            return;
        };
        if !status_bar.is_visible().unwrap_or(false) {
            continue;
        }
        let Ok(cursor) = app.cursor_position() else { continue };
        let monitors = match status_bar.available_monitors() {
            Ok(m) if !m.is_empty() => m,
            _ => continue,
        };
        let Some(idx) = monitor_for_cursor(&monitors, cursor) else { continue };
        let monitor = &monitors[idx];
        let name = monitor.name().cloned();
        if name != last_name {
            position_status_bar(&status_bar, monitor);
            // If the toast is currently visible, keep it pinned to the same
            // active monitor. We don't re-position when hidden because the
            // next `show_update_toast` call repositions on its own.
            if let Some(toast) = app.get_webview_window("update-toast") {
                if toast.is_visible().unwrap_or(false) {
                    position_update_toast(&toast, monitor);
                }
            }
            last_name = name;
        }
    }
}

/// Position the update-toast window above the pill on the given monitor,
/// horizontally centred over the pill. Pill anchor math mirrors the
/// `position_status_bar` formula so the two stay aligned.
pub fn position_update_toast(window: &tauri::WebviewWindow, monitor: &tauri::Monitor) {
    let scale = monitor.scale_factor();
    let size = monitor.size();
    let origin = monitor.position();
    let pill_w = (380.0 * scale) as i32;
    let pill_h = (72.0 * scale) as i32;
    let toast_w = (320.0 * scale) as i32;
    let toast_h = (96.0 * scale) as i32;
    let gap = (12.0 * scale) as i32;
    let pill_x = origin.x + (size.width as i32 - pill_w) / 2;
    let pill_y = origin.y + size.height as i32 - pill_h - (56.0 * scale) as i32;
    let x = pill_x + (pill_w - toast_w) / 2;
    let y = pill_y - toast_h - gap;
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Background auto-check entry point. Emits `updater://update-available`
/// to all windows when a new version is found and conditionally shows
/// the pill speech bubble (only when the user is not actively recording —
/// the about-nav-item dot still surfaces via the broadcast event).
async fn check_for_updates_silently(app: tauri::AppHandle) {
    use tauri_plugin_updater::UpdaterExt;
    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            log::warn!("auto-update: failed to get updater: {}", e);
            return;
        }
    };
    match updater.check().await {
        Ok(Some(update)) => {
            let payload = serde_json::json!({
                "version": update.version,
                "notes": update.body,
            });
            let _ = app.emit("updater://update-available", payload);
            let is_idle = matches!(
                *app.state::<AppStateManager>().0.lock().unwrap(),
                AppState::Idle
            );
            if is_idle {
                show_update_toast(&app, &update.version, update.body.as_deref());
            }
        }
        Ok(None) => {}
        Err(e) => {
            log::warn!("auto-update check failed: {}", e);
        }
    }
}

/// Render the speech-bubble notification above the pill. Re-positions to
/// the active monitor each time it is shown.
pub fn show_update_toast(app: &tauri::AppHandle, version: &str, notes: Option<&str>) {
    let Some(toast) = app.get_webview_window("update-toast") else {
        return;
    };
    let monitors = toast.available_monitors().unwrap_or_default();
    let cursor_monitor = app
        .cursor_position()
        .ok()
        .and_then(|c| monitor_for_cursor(&monitors, c))
        .and_then(|i| monitors.get(i).cloned());
    let monitor = cursor_monitor
        .or_else(|| toast.primary_monitor().ok().flatten())
        .or_else(|| toast.current_monitor().ok().flatten());
    if let Some(m) = &monitor {
        position_update_toast(&toast, m);
    }
    let payload = serde_json::json!({
        "version": version,
        "notes": notes,
    });
    let _ = toast.emit("update-toast://show", payload);
    let _ = toast.show();
    // Deliberately not calling set_focus — the toast is a notification,
    // not a workflow interruption. The user keeps focus on whatever they
    // were doing and clicks the toast only if they want to act.
}

pub fn update_tray_icon(app: &tauri::AppHandle, state: &AppState) {
    let bytes: &[u8] = match state {
        AppState::Recording  => include_bytes!("../icons/tray-recording.png"),
        AppState::Processing => include_bytes!("../icons/tray-processing.png"),
        AppState::Error      => include_bytes!("../icons/tray-error.png"),
        _                    => include_bytes!("../icons/tray-idle.png"),
    };
    if let Ok(icon) = Image::from_bytes(bytes) {
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_icon(Some(icon));
        }
    }
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit VOCA", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&settings_item, &separator, &quit_item])?;

    let initial_icon = Image::from_bytes(include_bytes!("../icons/tray-idle.png"))
        .unwrap_or_else(|_| app.default_window_icon().unwrap().clone());

    let builder = TrayIconBuilder::with_id("main")
        .icon(initial_icon)
        .menu(&menu)
        .tooltip("VOCA")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        });

    // Windows/Linux convention: left-click the tray icon to reveal the app.
    // On macOS the menu-bar extra should open the menu on click, so we leave
    // the default behaviour there.
    #[cfg(not(target_os = "macos"))]
    let builder = builder
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    builder.build(app)?;

    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
