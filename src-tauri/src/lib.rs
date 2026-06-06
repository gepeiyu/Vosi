pub mod app;
pub mod asr;
pub mod audio;
pub mod commands;
pub mod config;
pub mod hotkey;
pub mod inject;
pub mod log;
pub mod notify;
pub mod overlay;
pub mod pipeline;
pub mod post;

use app::state::AppState;
use app::tray::{self, TrayStatus};
use config::AppConfig;
use hotkey::listener::{self, HotkeyEvent};
use inject::{default_injector, inject_with_fallback, method_from_config};
use notify::Notifier;
use overlay::{OverlayController, OverlayState};
use log::Logger;
use asr::ModelManager;
use pipeline::session::VoiceSession;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
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

fn schedule_tray_reset(app: tauri::AppHandle, secs: u64) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(secs));
        tray::set_status(&app, TrayStatus::Idle);
    });
}

fn spawn_voice_pipeline(
    app: tauri::AppHandle,
    config: AppConfig,
    bundled: PathBuf,
    dev_models: Option<PathBuf>,
    logger: Arc<Logger>,
    overlay: OverlayController,
    notifier: Notifier,
) {
    let (hotkey_tx, hotkey_rx) = mpsc::channel::<HotkeyEvent>();
    listener::spawn_hotkey_listener(&config.hotkey.trigger_key, hotkey_tx);

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
                notifier.error("语音引擎不可用，请重新安装");
                tray::set_status(&app, TrayStatus::Warning);
                return;
            }
        };

        while let Ok(event) = hotkey_rx.recv() {
            match event {
                HotkeyEvent::Pressed => {
                    tray::set_status(&app, TrayStatus::Recording);
                    overlay.emit(OverlayState::Recording { level: 0.0 });
                    let (level_tx, level_rx) = mpsc::channel::<f32>();
                    let level_overlay = overlay.clone();
                    thread::spawn(move || {
                        while let Ok(level) = level_rx.recv() {
                            level_overlay.emit(OverlayState::Recording { level });
                        }
                    });
                    match session.on_hotkey_press_with_level(Some(level_tx)) {
                        Ok(()) => {}
                        Err(err) => {
                            logger.error(&format!("recording start failed: {err}"));
                            if err.contains("no input device") {
                                notifier.error("未检测到麦克风");
                                overlay.emit(OverlayState::Hidden);
                                tray::set_status(&app, TrayStatus::Warning);
                            } else {
                                overlay.emit(OverlayState::Hidden);
                                tray::set_status(&app, TrayStatus::Warning);
                            }
                        }
                    }
                }
                HotkeyEvent::Released => {
                    overlay.emit(OverlayState::Processing);
                    match session.on_hotkey_release() {
                        Ok(Some(text)) => {
                            let result =
                                inject_with_fallback(injector.as_ref(), &text, inject_method);
                            overlay.emit(OverlayState::Hidden);
                            if !result.injected {
                                notifier.error("已复制到剪贴板，请手动粘贴");
                                tray::set_status(&app, TrayStatus::Warning);
                                schedule_tray_reset(app.clone(), 3);
                            } else {
                                tray::set_status(&app, TrayStatus::Idle);
                            }
                        }
                        Ok(None) => {
                            overlay.emit(OverlayState::Hidden);
                            tray::set_status(&app, TrayStatus::Idle);
                        }
                        Err(err) => {
                            logger.error(&format!("voice pipeline error: {err}"));
                            overlay.emit(OverlayState::Hidden);
                            if err.contains("timeout") {
                                notifier.error("识别超时，请重试");
                            }
                            tray::set_status(&app, TrayStatus::Warning);
                            if err.contains("timeout") {
                                schedule_tray_reset(app.clone(), 3);
                            }
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
            let overlay = OverlayController::new(
                app.handle().clone(),
                config.overlay.enabled,
            );
            let notifier = Notifier::new(app.handle().clone());
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
                overlay,
                notifier,
            );
            Ok(())
        })
        .on_menu_event(|app, event| {
            let id = event.id().0.as_str();
            match id {
                "show" => tray::show_settings_window(app),
                "quit" => app.exit(0),
                other => eprintln!("unhandled tray menu item: {other}"),
            }
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
