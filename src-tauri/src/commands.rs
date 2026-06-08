use crate::app::state::AppState;
use crate::config::AppConfig;
use crate::permissions::PermissionsSnapshot;

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> AppConfig {
    state.get_config()
}

#[tauri::command]
pub fn save_config(state: tauri::State<'_, AppState>, cfg: AppConfig) -> Result<(), String> {
    state.set_config(cfg)
}

#[tauri::command]
pub fn get_permissions_status(state: tauri::State<'_, AppState>) -> PermissionsSnapshot {
    crate::permissions::snapshot(state.inner())
}

#[tauri::command]
pub fn open_permission_settings(permission_id: String) -> Result<(), String> {
    crate::permissions::open_settings(&permission_id)
}

#[tauri::command]
pub fn recheck_permissions(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> PermissionsSnapshot {
    #[cfg(target_os = "macos")]
    {
        use crate::permissions::microphone_macos::{is_accessibility_trusted, repair_accessibility};
        if !is_accessibility_trusted() {
            let _ = repair_accessibility(true);
        }
    }
    if crate::permissions::all_granted() {
        let _ = crate::try_start_voice_pipeline(&app, state.inner());
    } else if let Some(logger) = state.logger() {
        logger.info("recheck: permissions still incomplete");
    }
    crate::permissions::snapshot(state.inner())
}
