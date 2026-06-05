use crate::asr::paths::resolve_punctuation_model;
use sherpa_onnx::{OfflinePunctuation, OfflinePunctuationConfig, OfflinePunctuationModelConfig};
use std::path::Path;

pub struct PunctuationEngine {
    engine: OfflinePunctuation,
}

impl PunctuationEngine {
    pub fn new(punc_dir: &Path, num_threads: i32) -> Result<Self, String> {
        let model = resolve_punctuation_model(punc_dir)?;

        let mut config = OfflinePunctuationConfig::default();
        config.model = OfflinePunctuationModelConfig {
            ct_transformer: Some(model.to_string_lossy().into_owned()),
            num_threads,
            debug: false,
            provider: Some("cpu".into()),
        };

        let engine = OfflinePunctuation::create(&config)
            .ok_or_else(|| "failed to create OfflinePunctuation — check model files".to_string())?;

        Ok(Self { engine })
    }

    pub fn punctuate(&self, text: &str) -> String {
        if text.trim().is_empty() {
            return String::new();
        }
        self.engine
            .add_punctuation(text)
            .unwrap_or_else(|| text.to_string())
    }
}
