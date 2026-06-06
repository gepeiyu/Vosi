use tauri::image::Image;
use tauri::{include_image, ActivationPolicy, AppHandle, Manager, Runtime};

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

fn icon_for(status: TrayStatus) -> Image<'static> {
    match status {
        TrayStatus::Idle => include_image!("icons/icon-idle.png"),
        TrayStatus::Recording => include_image!("icons/icon-recording.png"),
        TrayStatus::Warning => include_image!("icons/icon-warning.png"),
    }
}

pub fn set_status<R: Runtime>(app: &AppHandle<R>, status: TrayStatus) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip_for(status)));
        let _ = tray.set_icon(Some(icon_for(status)));
    }
}

pub fn show_settings_window<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(ActivationPolicy::Regular);
        let _ = app.show();
    }

    let Some(window) = app.get_webview_window("main") else {
        eprintln!("settings window `main` not found");
        return;
    };

    let _ = window.unminimize();
    if let Err(err) = window.show() {
        eprintln!("settings window show failed: {err}");
        return;
    }
    let _ = window.set_focus();
}
