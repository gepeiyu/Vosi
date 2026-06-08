pub mod types;
pub use types::AppConfig;

use std::fs;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    dirs::config_dir().expect("config dir").join("vosi")
}

pub fn config_path() -> PathBuf {
    config_dir().join("settings.toml")
}

pub(crate) fn migrate_config(mut cfg: AppConfig) -> AppConfig {
    // 旧版跨平台默认 RightAlt → 各平台推荐键
    if cfg.hotkey.trigger_key == "RightAlt" && cfg!(target_os = "macos") {
        cfg.hotkey.trigger_key = types::default_trigger_key();
    }
    cfg
}

pub fn load() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        let cfg = AppConfig::default();
        save(&cfg).expect("save default config");
        return cfg;
    }
    let raw = fs::read_to_string(&path).expect("read config");
    let cfg: AppConfig = toml::from_str(&raw).expect("parse config");
    let cfg = migrate_config(cfg);
    let _ = save(&cfg);
    cfg
}

pub fn save(cfg: &AppConfig) -> std::io::Result<()> {
    fs::create_dir_all(config_dir())?;
    let raw = toml::to_string_pretty(cfg).expect("serialize config");
    fs::write(config_path(), raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_round_trips_through_toml() {
        let cfg = AppConfig::default();
        let raw = toml::to_string(&cfg).unwrap();
        let parsed: AppConfig = toml::from_str(&raw).unwrap();
        assert_eq!(cfg, parsed);
    }

    #[test]
    fn config_without_overlay_section_gets_default_overlay() {
        let raw = r#"
[hotkey]
trigger_key = "RightCommand"
mode = "hold"

[audio]
sample_rate = 16000
silence_threshold_ms = 800
min_speech_ms = 300

[asr]
num_threads = 2
mode = "short"
model_variant = "sense-voice-int8"

[hotword]
enabled = true
file = "~/.config/vosi/hotwords.txt"

[inject]
method = "type"

[general]
start_on_boot = true
show_tray = true
"#;
        let cfg: AppConfig = toml::from_str(raw).unwrap();
        let migrated = migrate_config(cfg);
        assert_eq!(migrated.overlay.enabled, true);
    }
}
