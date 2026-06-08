use crate::asr::paths::resolve_sense_voice_paths;
use sherpa_onnx::{
    OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig,
};
use std::path::Path;

pub struct AsrEngineOptions {
    pub language: String,
    pub use_itn: bool,
}

pub struct AsrEngine {
    recognizer: OfflineRecognizer,
}

impl AsrEngine {
    pub fn new(
        sense_voice_dir: &Path,
        num_threads: i32,
        options: AsrEngineOptions,
    ) -> Result<Self, String> {
        let (model, tokens) = resolve_sense_voice_paths(sense_voice_dir)?;

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.sense_voice = OfflineSenseVoiceModelConfig {
            model: Some(model.to_string_lossy().into_owned()),
            language: Some(options.language),
            use_itn: options.use_itn,
        };
        config.model_config.tokens = Some(tokens.to_string_lossy().into_owned());
        config.model_config.num_threads = num_threads;
        config.model_config.provider = Some("cpu".into());

        let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
            "failed to create OfflineRecognizer — check sense-voice model files".to_string()
        })?;

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
