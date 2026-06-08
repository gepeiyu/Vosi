use super::microphone_macos::{
    activate_app, is_accessibility_trusted, is_microphone_authorized, microphone_status,
    open_privacy_settings, prompt_microphone_denied, request_accessibility,
    request_microphone_permission, MicrophoneStatus,
};
use super::status::{PermissionsSnapshot, SetupPhase};
use crate::app::state::AppState;
use crate::app::tray::{self, TrayStatus};
use crate::log::Logger;
use std::sync::Arc;
use tauri::AppHandle;

pub fn all_granted() -> bool {
    is_microphone_authorized() && is_accessibility_trusted()
}

pub fn snapshot(state: &AppState) -> PermissionsSnapshot {
    let voice_ready = state.voice_ready();
    let mic_status = microphone_status();
    let permissions = vec![
        super::status::PermissionState {
            id: "microphone".into(),
            label: "麦克风".into(),
            description: "录制语音".into(),
            granted: is_microphone_authorized(),
            action_label: if is_microphone_authorized() {
                String::new()
            } else if mic_status == MicrophoneStatus::NotDetermined {
                "请求授权".into()
            } else {
                "去设置".into()
            },
        },
        super::status::PermissionState {
            id: "accessibility".into(),
            label: "辅助功能".into(),
            description: "监听热键与输入文字".into(),
            granted: is_accessibility_trusted(),
            action_label: if is_accessibility_trusted() {
                String::new()
            } else {
                "去设置".into()
            },
        },
    ];
    let all_granted = permissions.iter().all(|p| p.granted);
    let (setup_phase, setup_message) = if voice_ready {
        (SetupPhase::Ready, None)
    } else if !all_granted {
        (SetupPhase::WaitingPermissions, None)
    } else {
        (state.setup_phase(), state.setup_message())
    };

    PermissionsSnapshot {
        all_granted,
        voice_ready,
        setup_phase,
        setup_message,
        permissions,
        reinstall_tip: if all_granted {
            None
        } else {
            Some(
                "若已在系统设置中开启仍无效：关闭再打开 Vosi 开关，或移除后重新添加，然后点击「重新检查权限」。".into(),
            )
        },
    }
}

fn log_permissions(logger: &Logger, state: &AppState) {
    let snap = snapshot(state);
    for item in &snap.permissions {
        let state = if item.granted { "ok" } else { "missing" };
        logger.info(&format!("permission {}: {state}", item.id));
    }
}

pub fn open_settings(permission_id: &str) -> Result<(), String> {
    activate_app();
    match permission_id {
        "microphone" => {
            if is_microphone_authorized() {
                return Ok(());
            }
            match microphone_status() {
                MicrophoneStatus::NotDetermined => {
                    let _ = request_microphone_permission();
                }
                MicrophoneStatus::Denied | MicrophoneStatus::Restricted => {
                    open_privacy_settings("Privacy_Microphone")?;
                }
                MicrophoneStatus::Authorized => {}
            }
        }
        "accessibility" => {
            if is_accessibility_trusted() {
                return Ok(());
            }
            let _ = request_accessibility();
            if !is_accessibility_trusted() {
                open_privacy_settings("Privacy_Accessibility")?;
            }
        }
        "input_monitoring" => {
            // Legacy id: input monitoring is covered by accessibility on modern macOS.
            if is_accessibility_trusted() {
                return Ok(());
            }
            let _ = request_accessibility();
            open_privacy_settings("Privacy_Accessibility")?;
        }
        other => return Err(format!("unknown permission: {other}")),
    }
    Ok(())
}

/// Run on the main thread during startup.
/// Returns true when all permissions are granted and the app may start.
/// Returns false when permissions are incomplete.
pub fn run_startup_gate(app: &AppHandle, logger: Arc<Logger>, state: &AppState) -> bool {
    log_permissions(&logger, state);

    if all_granted() {
        logger.info("permissions ready");
        tray::set_status(app, TrayStatus::Idle);
        return true;
    }

    tray::set_status(app, TrayStatus::Warning);
    logger.info("permissions incomplete — running startup gate");
    activate_app();

    if !is_microphone_authorized() {
        logger.info(&format!(
            "requesting microphone permission (status={:?})",
            microphone_status()
        ));
        match microphone_status() {
            MicrophoneStatus::NotDetermined => {
                if !request_microphone_permission() {
                    logger.info("microphone permission not granted");
                    return false;
                }
            }
            MicrophoneStatus::Denied | MicrophoneStatus::Restricted => {
                logger.info("microphone permission denied — opening settings");
                let _ = prompt_microphone_denied();
                return false;
            }
            MicrophoneStatus::Authorized => {}
        }
    }

    if !is_accessibility_trusted() {
        logger.info("requesting accessibility permission");
        let _ = request_accessibility();
        if is_accessibility_trusted() {
            logger.info("accessibility permission granted");
            tray::set_status(app, TrayStatus::Idle);
            return true;
        }
        logger.info("accessibility permission not granted");
        return false;
    }

    tray::set_status(app, TrayStatus::Idle);
    true
}
