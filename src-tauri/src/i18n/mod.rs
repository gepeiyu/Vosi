pub mod locale;

use locale::Locale;
use std::collections::HashMap;
use std::sync::LazyLock;
use tauri::{AppHandle, Emitter, Manager, Runtime};

static CATALOG: LazyLock<HashMap<Locale, HashMap<String, String>>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert(Locale::Zh, parse(include_str!("../locales/zh.json")));
    map.insert(Locale::En, parse(include_str!("../locales/en.json")));
    map.insert(Locale::Ja, parse(include_str!("../locales/ja.json")));
    map
});

fn parse(raw: &str) -> HashMap<String, String> {
    serde_json::from_str(raw).expect("parse locale json")
}

pub fn t(locale: Locale, key: &str) -> String {
    t_with_vars(locale, key, &[])
}

pub fn t_with_vars(locale: Locale, key: &str, vars: &[(&str, &str)]) -> String {
    let text = lookup(locale, key);
    interpolate(text, vars)
}

fn lookup(locale: Locale, key: &str) -> String {
    CATALOG
        .get(&locale)
        .and_then(|c| c.get(key))
        .or_else(|| CATALOG.get(&Locale::Zh).and_then(|c| c.get(key)))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("missing i18n key: {key}");
            key.to_string()
        })
}

fn interpolate(mut text: String, vars: &[(&str, &str)]) -> String {
    for (name, value) in vars {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

pub fn apply_locale<R: Runtime>(app: &AppHandle<R>, locale: Locale) {
    let _ = crate::app::tray::rebuild_tray_menu(app, locale);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_title(&t(locale, "window.main.title"));
    }
    if let Some(w) = app.get_webview_window("about") {
        let _ = w.set_title(&t(locale, "window.about.title"));
    }
    let _ = app.emit("locale-changed", locale.as_str());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_tray_keys_exist_in_all_locales() {
        for locale in [Locale::Zh, Locale::En, Locale::Ja] {
            assert!(!t(locale, "tray.menu.settings").is_empty());
            assert!(!t(locale, "tray.menu.about").is_empty());
            assert!(!t(locale, "tray.menu.quit").is_empty());
        }
    }

    #[test]
    fn interpolation_replaces_vars() {
        let msg = t_with_vars(
            Locale::En,
            "setup.models_install_failed",
            &[("error", "disk full")],
        );
        assert!(msg.contains("disk full"));
    }
}
