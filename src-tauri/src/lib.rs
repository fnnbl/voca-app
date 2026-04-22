pub mod ai;
pub mod audio;
pub mod clipboard;
pub mod commands;
pub mod errors;
pub mod hotkey;
pub mod keychain;
pub mod local_transcription;
pub mod shortcut;
pub mod stats;
pub mod storage;
pub mod target_app;
pub mod transcription;

use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
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
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(AppStateManager(Mutex::new(AppState::Idle)))
        .manage(AudioRecordingState::new())
        .manage(AudioBuffer(Mutex::new(None)))
        .manage(WhisperModelCache(Arc::new(Mutex::new(None))))
        .manage(LastPressTime(Mutex::new(None)))
        .manage(LastTapTime(Mutex::new(None)))
        .manage(IsToggleSession(Mutex::new(false)))
        .manage(SelectedAudioDevice(Mutex::new(None)))
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

            if let Some(status_bar) = app.get_webview_window("status-bar") {
                // Prefer primary_monitor for reliable startup positioning;
                // the monitor's .position() offset is added so multi-monitor
                // setups where the primary isn't at (0,0) work correctly.
                let monitor = status_bar
                    .primary_monitor()
                    .ok()
                    .flatten()
                    .or_else(|| status_bar.current_monitor().ok().flatten());
                if let Some(m) = monitor {
                    position_status_bar(&status_bar, &m);
                }
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
            commands::get_snippets,
            commands::save_snippets,
            commands::get_autostart,
            commands::set_autostart,
            commands::get_history,
            commands::clear_history,
            commands::get_stats,
            commands::set_window_theme,
            commands::list_audio_devices,
            commands::set_audio_device,
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

    let builder = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
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
