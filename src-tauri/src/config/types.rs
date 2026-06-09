#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub hotkey: HotkeyConfig,
    pub audio: AudioConfig,
    pub asr: AsrConfig,
    pub hotword: HotwordConfig,
    pub inject: InjectConfig,
    pub general: GeneralConfig,
    #[serde(default)]
    pub overlay: OverlayConfig,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HotkeyConfig {
    pub trigger_key: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub silence_threshold_ms: u32,
    pub min_speech_ms: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AsrConfig {
    pub num_threads: u32,
    pub mode: String,
    pub model_variant: String,
    #[serde(default = "default_asr_language")]
    pub language: String,
    #[serde(default = "default_use_itn")]
    pub use_itn: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HotwordConfig {
    pub enabled: bool,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InjectConfig {
    pub method: String,
}

fn default_locale() -> String {
    "zh".into()
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeneralConfig {
    pub start_on_boot: bool,
    pub show_tray: bool,
    #[serde(default = "default_locale")]
    pub locale: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OverlayConfig {
    pub enabled: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

fn default_asr_language() -> String {
    "auto".into()
}

fn default_use_itn() -> bool {
    true
}

/// macOS: 空格右侧 Command；Windows: 空格右侧 Alt
pub fn default_trigger_key() -> String {
    if cfg!(target_os = "macos") {
        "RightCommand".into()
    } else {
        "RightAlt".into()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: HotkeyConfig {
                trigger_key: default_trigger_key(),
                mode: "hold".into(),
            },
            audio: AudioConfig {
                sample_rate: 16000,
                silence_threshold_ms: 800,
                min_speech_ms: 300,
            },
            asr: AsrConfig {
                num_threads: 2,
                mode: "short".into(),
                model_variant: "sense-voice-int8".into(),
                language: default_asr_language(),
                use_itn: default_use_itn(),
            },
            hotword: HotwordConfig {
                enabled: true,
                file: "~/.config/vosi/hotwords.txt".into(),
            },
            inject: InjectConfig {
                method: "type".into(),
            },
            general: GeneralConfig {
                start_on_boot: true,
                show_tray: true,
                locale: default_locale(),
            },
            overlay: OverlayConfig { enabled: true },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asr_config_deserializes_with_defaults_for_new_fields() {
        let raw = r#"
num_threads = 2
mode = "short"
model_variant = "sense-voice-int8"
"#;
        #[derive(serde::Deserialize)]
        struct Wrapper {
            asr: AsrConfig,
        }
        let cfg: Wrapper = toml::from_str(&format!("[asr]\n{raw}")).unwrap();
        assert_eq!(cfg.asr.language, "auto");
        assert!(cfg.asr.use_itn);
    }

    #[test]
    fn asr_config_default_values() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.asr.model_variant, "sense-voice-int8");
        assert_eq!(cfg.asr.language, "auto");
        assert!(cfg.asr.use_itn);
    }
}
