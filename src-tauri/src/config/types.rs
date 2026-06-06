#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub hotkey: HotkeyConfig,
    pub audio: AudioConfig,
    pub asr: AsrConfig,
    pub hotword: HotwordConfig,
    pub inject: InjectConfig,
    pub general: GeneralConfig,
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeneralConfig {
    pub start_on_boot: bool,
    pub show_tray: bool,
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
                model_variant: "paraformer-large-int8".into(),
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
            },
        }
    }
}
