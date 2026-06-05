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

pub fn load() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        let cfg = AppConfig::default();
        save(&cfg).expect("save default config");
        return cfg;
    }
    let raw = fs::read_to_string(&path).expect("read config");
    toml::from_str(&raw).expect("parse config")
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
}
