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
        let mut hint = String::from(
            "Vosi 需要在「系统设置 → 隐私与安全性」中授权「辅助功能」和「麦克风」。\
             开发模式（tauri dev）不会显示「Vosi」，请添加本进程可执行文件：",
        );
        if let Ok(exe) = std::env::current_exe() {
            hint.push('\n');
            hint.push_str(&exe.display().to_string());
        }
        hint.push_str(
            "\n\n热键监听还需要「辅助功能」权限（CGEventTap）。\
             也可先按住触发键说话，系统弹出麦克风授权时再点允许。\
             辅助功能需点「+」手动添加上述路径。",
        );
        Some(hint)
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
