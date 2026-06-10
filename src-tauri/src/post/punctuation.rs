use sherpa_onnx::{OfflinePunctuation, OfflinePunctuationConfig};
use std::path::Path;

pub struct PunctuationEngine {
    engine: OfflinePunctuation,
}

impl PunctuationEngine {
    pub fn new(model_path: &Path, num_threads: i32) -> Result<Self, String> {
        let mut config = OfflinePunctuationConfig::default();
        config.model.ct_transformer = Some(model_path.to_string_lossy().into_owned());
        config.model.num_threads = num_threads;
        config.model.provider = Some("cpu".into());

        let engine = OfflinePunctuation::create(&config)
            .ok_or_else(|| "failed to create punctuation engine".to_string())?;
        Ok(Self { engine })
    }

    pub fn punctuate(&self, text: &str) -> Result<String, String> {
        self.engine
            .add_punctuation(text)
            .ok_or_else(|| "punctuation inference failed".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonexistent_model_fails_to_load() {
        assert!(PunctuationEngine::new(Path::new("/nonexistent/punctuation.onnx"), 1).is_err());
    }
}
