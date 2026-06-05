use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayStatus {
    Idle,
    Recording,
    Warning,
}

pub fn tooltip_for(status: TrayStatus) -> &'static str {
    match status {
        TrayStatus::Idle => "Vosi — 就绪",
        TrayStatus::Recording => "Vosi — 正在录音…",
        TrayStatus::Warning => "Vosi — 需要辅助功能权限",
    }
}

pub fn set_status<R: Runtime>(app: &AppHandle<R>, status: TrayStatus) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip_for(status)));
    }
}

pub fn show_settings_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
