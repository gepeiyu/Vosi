use crate::asr::paths::resolve_paraformer_paths;
use sherpa_onnx::{OfflineParaformerModelConfig, OfflineRecognizer, OfflineRecognizerConfig};
use std::path::Path;

pub struct AsrEngine {
    recognizer: OfflineRecognizer,
}

impl AsrEngine {
    pub fn new(paraformer_dir: &Path, num_threads: i32) -> Result<Self, String> {
        let (model, tokens) = resolve_paraformer_paths(paraformer_dir)?;

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.paraformer = OfflineParaformerModelConfig {
            model: Some(model.to_string_lossy().into_owned()),
        };
        config.model_config.tokens = Some(tokens.to_string_lossy().into_owned());
        config.model_config.num_threads = num_threads;
        config.model_config.provider = Some("cpu".into());

        let recognizer = OfflineRecognizer::create(&config)
            .ok_or_else(|| "failed to create OfflineRecognizer — check model files".to_string())?;

        Ok(Self { recognizer })
    }

    pub fn transcribe(&self, samples: &[f32], sample_rate: u32) -> String {
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(sample_rate as i32, samples);
        self.recognizer.decode(&stream);
        stream
            .get_result()
            .map(|r| r.text)
            .unwrap_or_default()
    }
}
