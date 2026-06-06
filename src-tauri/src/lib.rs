pub mod app;
pub mod asr;
pub mod audio;
pub mod commands;
pub mod config;
pub mod hotkey;
pub mod inject;
pub mod log;
pub mod notify;
pub mod pipeline;
pub mod post;

use app::state::AppState;
use app::tray::{self, TrayStatus};
use config::AppConfig;
use hotkey::listener::{self, HotkeyEvent};
use inject::{default_injector, method_from_config};
use log::Logger;
use asr::ModelManager;
use pipeline::session::VoiceSession;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::thread;
use tauri::menu::{Menu, MenuItem};
use tauri::Manager;

fn models_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vosi")
}

fn dev_models_dir() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    {
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models/dev");
        if ModelManager::paraformer_ready(&dev) {
            return Some(dev);
        }
    }
    None
}

fn spawn_voice_pipeline(
    app: tauri::AppHandle,
    config: AppConfig,
    bundled: PathBuf,
    dev_models: Option<PathBuf>,
    logger: Arc<Logger>,
) {
    let (hotkey_tx, hotkey_rx) = mpsc::channel::<HotkeyEvent>();
    let trigger = listener::key_from_name(&config.hotkey.trigger_key);
    listener::spawn_hotkey_listener(trigger, hotkey_tx);

    let inject_method = method_from_config(&config.inject.method);
    thread::spawn(move || {
        let injector = default_injector();
        let mut session = match VoiceSession::try_new(
            config,
            models_data_dir(),
            &bundled,
            dev_models.as_deref(),
            logger.clone(),
        ) {
            Ok(session) => session,
            Err(err) => {
                logger.error(&format!("voice pipeline unavailable: {err}"));
                tray::set_status(&app, TrayStatus::Warning);
                return;
            }
        };

        while let Ok(event) = hotkey_rx.recv() {
            match event {
                HotkeyEvent::Pressed => {
                    tray::set_status(&app, TrayStatus::Recording);
                    if let Err(err) = session.on_hotkey_press() {
                        logger.error(&format!("recording start failed: {err}"));
                        tray::set_status(&app, TrayStatus::Warning);
                    }
                }
                HotkeyEvent::Released => {
                    tray::set_status(&app, TrayStatus::Idle);
                    match session.on_hotkey_release() {
                        Ok(Some(text)) => {
                            if let Err(err) = injector.inject(&text, inject_method) {
                                logger.error(&format!("text injection failed: {err}"));
                                tray::set_status(&app, TrayStatus::Warning);
                            }
                        }
                        Ok(None) => {}
                        Err(err) => {
                            logger.error(&format!("voice pipeline error: {err}"));
                            tray::set_status(&app, TrayStatus::Warning);
                        }
                    }
                }
            }
        }
    });
}

fn setup_tray_menu(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
    }
    tray::set_status(app.handle(), TrayStatus::Idle);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = config::load();
    let app_state = AppState::new(config.clone());
    let logger = Arc::new(
        Logger::default_logger().unwrap_or_else(|err| {
            eprintln!("logger init failed: {err}");
            Logger::new(std::env::temp_dir().join("vosi-logs")).expect("fallback logger")
        }),
    );
    logger.info("vosi starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(app_state)
        .setup(move |app| {
            setup_tray_menu(app)?;
            let bundled = app
                .path()
                .resource_dir()
                .map(|dir| dir.join("models/bundled"))
                .unwrap_or_else(|_| PathBuf::from("models/bundled"));
            spawn_voice_pipeline(
                app.handle().clone(),
                config,
                bundled,
                dev_models_dir(),
                logger,
            );
            Ok(())
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => tray::show_settings_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::get_accessibility_hint,
            commands::open_accessibility_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
