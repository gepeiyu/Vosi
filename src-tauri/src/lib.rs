pub mod app;
pub mod asr;
pub mod audio;
pub mod commands;
pub mod config;
pub mod hotkey;
pub mod i18n;
pub mod inject;
pub mod log;
pub mod notify;
pub mod overlay;
pub mod permissions;
pub mod pipeline;
pub mod post;

use app::state::AppState;
use app::tray::{self, TrayStatus};
use asr::ModelManager;
use permissions::SetupPhase;
use config::AppConfig;
use hotkey::listener::{self, HotkeyEvent};
use crate::i18n::t_with_vars;
use inject::{default_injector, inject_with_fallback, method_from_config};
use notify::Notifier;
use overlay::{OverlayController, OverlayState};
use log::Logger;
use pipeline::session::VoiceSession;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

const MIN_HOLD_MS: u64 = 300;

enum PipelineEvent {
    Hotkey(HotkeyEvent),
}

fn begin_recording(
    session: &mut VoiceSession,
    app: &tauri::AppHandle,
    overlay: &OverlayController,
    level_updates_active: &mut Option<Arc<AtomicBool>>,
    logger: &Logger,
    notifier: &Notifier,
) -> Result<(), String> {
    tray::set_status(app, TrayStatus::Recording);
    overlay.emit(OverlayState::Recording { level: 0.0 });
    let level_active = Arc::new(AtomicBool::new(true));
    *level_updates_active = Some(Arc::clone(&level_active));
    let (level_tx, level_rx) = mpsc::channel::<f32>();
    let level_overlay = overlay.clone();
    thread::spawn(move || {
        while let Ok(level) = level_rx.recv() {
            if !level_active.load(Ordering::Relaxed) {
                break;
            }
            level_overlay.emit(OverlayState::Recording { level });
        }
    });
    if let Err(err) = session.on_hotkey_press_with_level(Some(level_tx)) {
        if let Some(active) = level_updates_active.take() {
            active.store(false, Ordering::Relaxed);
        }
        logger.error(&format!("recording start failed: {err}"));
        if err.contains("no input device") {
            let locale = app.state::<AppState>().locale();
            notifier.error(&i18n::t(locale, "notify.mic_unavailable"));
        }
        overlay.emit(OverlayState::Hidden);
        tray::set_status(app, TrayStatus::Warning);
        return Err(err);
    }
    Ok(())
}

fn stop_level_updates(level_updates_active: &mut Option<Arc<AtomicBool>>) {
    if let Some(active) = level_updates_active.take() {
        active.store(false, Ordering::Relaxed);
    }
}
use tauri::{Emitter, Manager, WindowEvent};

fn models_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vosi")
}

fn dev_models_dir() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    {
        let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models/dev");
        if crate::asr::ModelManager::sense_voice_ready(&dev)
            || crate::asr::ModelManager::punctuation_ready(&dev)
        {
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
    let (pipeline_tx, pipeline_rx) = mpsc::channel::<PipelineEvent>();
    let hotkey_pipeline_tx = pipeline_tx.clone();
    listener::spawn_hotkey_listener(app.state::<AppState>().config_handle(), hotkey_tx);

    thread::spawn(move || {
        for event in hotkey_rx {
            let _ = hotkey_pipeline_tx.send(PipelineEvent::Hotkey(event));
        }
    });

    let inject_method = method_from_config(&config.inject.method);
    thread::spawn(move || {
        let state = app.state::<AppState>();
        let models_root = models_data_dir();
        let mgr = ModelManager::new(models_root.clone());

        if ModelManager::needs_install(&models_root, &bundled) {
            let locale = state.locale();
            state.set_setup(
                SetupPhase::InstallingModels,
                Some(i18n::t(locale, "setup.installing_models")),
            );
            let _ = app.emit("setup-updated", ());
            logger.info("installing bundled speech models");
            tray::set_status(&app, TrayStatus::Warning);
            if let Err(err) = mgr.ensure_installed(&bundled, dev_models.as_deref()) {
                let locale = state.locale();
                let msg = t_with_vars(
                    locale,
                    "setup.models_install_failed",
                    &[("error", &err.to_string())],
                );
                logger.error(&msg);
                state.set_setup(SetupPhase::Error, Some(msg));
                state.clear_pipeline_spawned();
                let _ = app.emit("setup-updated", ());
                notifier.error(&i18n::t(locale, "notify.engine_unavailable"));
                return;
            }
            logger.info("speech models installed");
        }

        let locale = state.locale();
        state.set_setup(
            SetupPhase::LoadingEngine,
            Some(i18n::t(locale, "setup.loading_engine")),
        );
        let _ = app.emit("setup-updated", ());

        let injector = default_injector();
        let mut session = match VoiceSession::try_new(
            config,
            models_root,
            &bundled,
            dev_models.as_deref(),
            logger.clone(),
        ) {
            Ok(session) => session,
            Err(err) => {
                logger.error(&format!("voice pipeline unavailable: {err}"));
                let locale = state.locale();
                state.set_setup(
                    SetupPhase::Error,
                    Some(i18n::t(locale, "setup.engine_load_failed")),
                );
                state.clear_pipeline_spawned();
                let _ = app.emit("setup-updated", ());
                notifier.error(&i18n::t(locale, "notify.engine_unavailable"));
                tray::set_status(&app, TrayStatus::Warning);
                return;
            }
        };

        state.mark_pipeline_started();
        let _ = app.emit("setup-updated", ());
        tray::set_status(&app, TrayStatus::Idle);
        logger.info("voice pipeline ready");

        let mut level_updates_active: Option<Arc<AtomicBool>> = None;
        let mut press_started: Option<Instant> = None;

        while let Ok(event) = pipeline_rx.recv() {
            match event {
                PipelineEvent::Hotkey(HotkeyEvent::Pressed) => {
                    if session.is_recording() {
                        continue;
                    }
                    logger.info("hotkey pressed");
                    press_started = Some(Instant::now());
                    if begin_recording(
                        &mut session,
                        &app,
                        &overlay,
                        &mut level_updates_active,
                        &logger,
                        &notifier,
                    )
                    .is_err()
                    {
                        press_started = None;
                    }
                }
                PipelineEvent::Hotkey(HotkeyEvent::Released) => {
                    let held_ms = press_started
                        .take()
                        .map(|t| t.elapsed().as_millis())
                        .unwrap_or(0);

                    if held_ms < MIN_HOLD_MS as u128 {
                        if session.is_recording() {
                            session.cancel_recording();
                            stop_level_updates(&mut level_updates_active);
                            overlay.emit(OverlayState::Hidden);
                            tray::set_status(&app, TrayStatus::Idle);
                        }
                        logger.info("hotkey tap ignored");
                        continue;
                    }

                    if !session.is_recording() {
                        logger.info("hotkey released without active recording");
                        continue;
                    }

                    stop_level_updates(&mut level_updates_active);

                    logger.info("hotkey released");
                    overlay.emit(OverlayState::Processing);
                    match session.on_hotkey_release() {
                        Ok(Some(text)) => {
                            let result =
                                inject_with_fallback(injector.as_ref(), &text, inject_method);
                            overlay.emit(OverlayState::Hidden);
                            if !result.injected {
                                let detail = result.error.as_deref().unwrap_or("unknown");
                                logger.error(&format!("text injection failed: {detail}"));
                                if cfg!(target_os = "macos")
                                    && !crate::permissions::is_accessibility_trusted()
                                {
                                    let locale = app.state::<AppState>().locale();
                                    notifier.error(&i18n::t(
                                        locale,
                                        "notify.inject_accessibility_failed",
                                    ));
                                } else {
                                    let locale = app.state::<AppState>().locale();
                                    notifier.error(&i18n::t(locale, "notify.clipboard_fallback"));
                                }
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
                                let locale = app.state::<AppState>().locale();
                                notifier.error(&i18n::t(locale, "notify.recognition_timeout"));
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

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let locale = app.state::<AppState>().locale();
    tray::setup_tray(app.handle(), locale)?;
    Ok(())
}

pub fn try_start_voice_pipeline(app: &tauri::AppHandle, state: &AppState) -> bool {
    if state.voice_ready() {
        return true;
    }
    if state.pipeline_spawned() {
        return false;
    }
    if !permissions::all_granted() {
        return false;
    }
    let Some(logger) = state.logger() else {
        return false;
    };
    let Some(bundled) = state.bundled() else {
        return false;
    };
    if ModelManager::needs_install(&models_data_dir(), &bundled) {
        let locale = state.locale();
        state.set_setup(
            SetupPhase::InstallingModels,
            Some(i18n::t(locale, "setup.installing_models")),
        );
        tray::show_settings_window(app);
        tray::set_status(app, TrayStatus::Warning);
    } else {
        state.set_setup(
            SetupPhase::LoadingEngine,
            Some(i18n::t(state.locale(), "setup.loading_engine")),
        );
    }
    state.mark_pipeline_spawned();
    let config = state.get_config();
    let overlay = OverlayController::new(app.clone(), config.overlay.enabled);
    let notifier = Notifier::new(app.clone());
    spawn_voice_pipeline(
        app.clone(),
        config,
        bundled,
        state.dev_models(),
        logger,
        overlay,
        notifier,
    );
    true
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
            tray::configure_background_app(app.handle());
            setup_tray(app)?;
            i18n::apply_locale(app.handle(), app.state::<AppState>().locale());
            let bundled = app
                .path()
                .resource_dir()
                .map(|dir| dir.join("models/bundled"))
                .unwrap_or_else(|_| PathBuf::from("models/bundled"));
            app.state::<AppState>().init_runtime(
                logger.clone(),
                bundled,
                dev_models_dir(),
            );
            let app_state = app.state::<AppState>().inner();
            let ready = permissions::run_startup_gate(app.handle(), logger.clone(), app_state);
            if ready {
                try_start_voice_pipeline(app.handle(), app_state);
            } else {
                app_state.set_setup(SetupPhase::WaitingPermissions, None);
                logger.info("voice pipeline waiting for permissions");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if !matches!(window.label(), "main" | "about") {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                match window.label() {
                    "main" => tray::on_settings_close_requested(window),
                    "about" => tray::on_about_close_requested(window),
                    _ => {}
                }
            }
        })
        .on_menu_event(|app, event| {
            let id = event.id().0.as_str();
            match id {
                "show" => tray::show_settings_window(app),
                "about" => tray::show_about_window(app),
                "quit" => app.exit(0),
                other => eprintln!("unhandled tray menu item: {other}"),
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_config,
            commands::save_config,
            commands::get_permissions_status,
            commands::open_permission_settings,
            commands::recheck_permissions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
