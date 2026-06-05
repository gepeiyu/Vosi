use crate::app::state::AppState;
use crate::config::AppConfig;

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> AppConfig {
    state.get_config()
}

#[tauri::command]
pub fn save_config(state: tauri::State<'_, AppState>, cfg: AppConfig) -> Result<(), String> {
    state.set_config(cfg)
}

#[tauri::command]
pub fn get_accessibility_hint() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        Some(
            "Vosi 需要在「系统设置 → 隐私与安全性 → 辅助功能」中授权，才能将识别文字注入到当前应用。".into(),
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[tauri::command]
pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
