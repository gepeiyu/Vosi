use crate::i18n::{self, locale::Locale};
use std::sync::atomic::{AtomicU8, Ordering};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{include_image, AppHandle, Manager, Runtime, WebviewWindow, Window};

#[cfg(target_os = "macos")]
use crate::permissions::microphone_macos::activate_app;
#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayStatus {
    Idle,
    Recording,
    Warning,
}

impl TrayStatus {
    fn as_u8(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Recording => 1,
            Self::Warning => 2,
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Recording,
            2 => Self::Warning,
            _ => Self::Idle,
        }
    }
}

static TRAY_STATUS: AtomicU8 = AtomicU8::new(0);

pub fn tooltip_for(status: TrayStatus, locale: Locale) -> String {
    let key = match status {
        TrayStatus::Idle => "tray.tooltip.idle",
        TrayStatus::Recording => "tray.tooltip.recording",
        TrayStatus::Warning => "tray.tooltip.warning",
    };
    i18n::t(locale, key)
}

fn icon_for(status: TrayStatus) -> Image<'static> {
    match status {
        TrayStatus::Idle => include_image!("icons/icon-idle.png"),
        TrayStatus::Recording => include_image!("icons/icon-recording.png"),
        TrayStatus::Warning => include_image!("icons/icon-warning.png"),
    }
}

fn build_tray_menu<R: Runtime>(app: &AppHandle<R>, locale: Locale) -> Result<Menu<R>, tauri::Error> {
    let show = MenuItem::with_id(
        app,
        "show",
        i18n::t(locale, "tray.menu.settings"),
        true,
        None::<&str>,
    )?;
    let about = MenuItem::with_id(
        app,
        "about",
        i18n::t(locale, "tray.menu.about"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        i18n::t(locale, "tray.menu.quit"),
        true,
        None::<&str>,
    )?;
    Menu::with_items(app, &[&show, &about, &quit])
}

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>, locale: Locale) -> Result<(), tauri::Error> {
    let status = TrayStatus::from_u8(TRAY_STATUS.load(Ordering::Relaxed));
    let menu = build_tray_menu(app, locale)?;
    TrayIconBuilder::with_id("main")
        .icon(icon_for(status))
        .icon_as_template(false)
        .tooltip(tooltip_for(status, locale))
        .menu(&menu)
        .build(app)?;
    Ok(())
}

pub fn rebuild_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    locale: Locale,
) -> Result<(), tauri::Error> {
    let menu = build_tray_menu(app, locale)?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
        let status = TrayStatus::from_u8(TRAY_STATUS.load(Ordering::Relaxed));
        let _ = tray.set_tooltip(Some(tooltip_for(status, locale)));
    }
    Ok(())
}

pub fn set_status<R: Runtime>(app: &AppHandle<R>, status: TrayStatus) {
    TRAY_STATUS.store(status.as_u8(), Ordering::Relaxed);
    if let Some(tray) = app.tray_by_id("main") {
        let locale = app.state::<crate::app::state::AppState>().locale();
        let _ = tray.set_tooltip(Some(tooltip_for(status, locale)));
        let _ = tray.set_icon(Some(icon_for(status)));
    }
}

pub fn configure_background_app<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(ActivationPolicy::Accessory);
    }

    #[cfg(windows)]
    {
        hide_windows_from_taskbar(app);
    }
}

#[cfg(windows)]
fn hide_windows_from_taskbar<R: Runtime>(app: &AppHandle<R>) {
    for label in ["main", "about", "overlay"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_skip_taskbar(true);
        }
    }
}

pub fn show_settings_window<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(ActivationPolicy::Regular);
        activate_app();
        let _ = app.show();
    }

    let Some(window) = app.get_webview_window("main") else {
        eprintln!("settings window `main` not found");
        return;
    };

    show_settings_webview(&window);
}

pub fn show_settings_webview<R: Runtime>(window: &WebviewWindow<R>) {
    #[cfg(windows)]
    {
        let _ = window.set_skip_taskbar(true);
    }

    let _ = window.unminimize();
    let _ = window.center();
    if let Err(err) = window.show() {
        eprintln!("settings window show failed: {err}");
        return;
    }
    let _ = window.set_focus();
}

pub fn show_about_window<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(ActivationPolicy::Regular);
        activate_app();
        let _ = app.show();
    }

    let Some(window) = app.get_webview_window("about") else {
        eprintln!("about window `about` not found");
        return;
    };

    #[cfg(windows)]
    {
        let _ = window.set_skip_taskbar(true);
    }

    let _ = window.unminimize();
    let _ = window.center();
    if let Err(err) = window.show() {
        eprintln!("about window show failed: {err}");
        return;
    }
    let _ = window.set_focus();
}

fn restore_accessory_if_no_windows_visible<R: Runtime>(app: &AppHandle<R>) {
    let any_visible = ["main", "about"].iter().any(|label| {
        app.get_webview_window(label)
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false)
    });
    if !any_visible {
        let _ = app.set_activation_policy(ActivationPolicy::Accessory);
    }
}

/// Keep the settings window alive; hide instead of destroy (macOS tray apps).
pub fn on_settings_close_requested<R: Runtime>(window: &Window<R>) {
    let _ = window.hide();
    #[cfg(target_os = "macos")]
    {
        restore_accessory_if_no_windows_visible(window.app_handle());
    }
}

/// Keep the about window alive; hide instead of destroy (macOS tray apps).
pub fn on_about_close_requested<R: Runtime>(window: &Window<R>) {
    let _ = window.hide();
    #[cfg(target_os = "macos")]
    {
        restore_accessory_if_no_windows_visible(window.app_handle());
    }
}
