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
pub mod permissions;
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
use pipeline::session::VoiceSession;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

const MIN_HOLD_MS: u64 = 300;

enum PipelineEvent {
    Hotkey(HotkeyEvent),
    BeginRecording { press_gen: u64 },
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
            notifier.error("未检测到麦克风");
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
        if crate::asr::ModelManager::sense_voice_ready(&dev)
            || crate::asr::ModelManager::paraformer_ready(&dev)
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
    listener::spawn_hotkey_listener(&config.hotkey.trigger_key, hotkey_tx);

    thread::spawn(move || {
        for event in hotkey_rx {
            let _ = hotkey_pipeline_tx.send(PipelineEvent::Hotkey(event));
        }
    });

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

        let mut level_updates_active: Option<Arc<AtomicBool>> = None;
        let mut press_started: Option<Instant> = None;
        let mut press_gen: u64 = 0;

        while let Ok(event) = pipeline_rx.recv() {
            match event {
                PipelineEvent::Hotkey(HotkeyEvent::Pressed) => {
                    if session.is_recording() {
                        continue;
                    }
                    logger.info("hotkey pressed");
                    press_gen += 1;
                    let my_gen = press_gen;
                    press_started = Some(Instant::now());
                    let pipeline_tx = pipeline_tx.clone();
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(MIN_HOLD_MS));
                        let _ = pipeline_tx.send(PipelineEvent::BeginRecording { press_gen: my_gen });
                    });
                }
                PipelineEvent::BeginRecording { press_gen: gen } => {
                    if gen != press_gen || press_started.is_none() || session.is_recording() {
                        continue;
                    }
                    logger.info("hotkey hold threshold reached");
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
                        press_gen += 1;
                    }
                }
                PipelineEvent::Hotkey(HotkeyEvent::Released) => {
                    press_gen += 1;

                    let held_ms = press_started
                        .take()
                        .map(|t| t.elapsed().as_millis())
                        .unwrap_or(0);

                    if !session.is_recording() {
                        if held_ms < MIN_HOLD_MS as u128 {
                            logger.info("hotkey tap ignored");
                            continue;
                        }
                        logger.info("hotkey hold threshold reached on release");
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
                            continue;
                        }
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
                                    notifier.error(
                                        "文本注入失败：请重新授权辅助功能（关闭后重新打开 Vosi 开关）",
                                    );
                                } else {
                                    notifier.error("已复制到剪贴板，请手动粘贴");
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
    Ok(())
}

pub fn try_start_voice_pipeline(app: &tauri::AppHandle, state: &AppState) -> bool {
    if state.voice_ready() {
        return true;
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
    state.mark_pipeline_started();
    tray::set_status(app, TrayStatus::Idle);
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
            setup_tray_menu(app)?;
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
            let ready = permissions::run_startup_gate(app.handle(), logger.clone());
            if ready {
                try_start_voice_pipeline(app.handle(), app.state::<AppState>().inner());
            } else {
                logger.info("voice pipeline waiting for permissions");
            }
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
            commands::get_permissions_status,
            commands::open_permission_settings,
            commands::recheck_permissions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
