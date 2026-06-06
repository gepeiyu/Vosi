use crate::pipeline::session::meets_min_duration;
use sherpa_onnx::{SileroVadModelConfig, VadModelConfig, VoiceActivityDetector};
use std::path::Path;

pub struct VadEngine {
    detector: VoiceActivityDetector,
    sample_rate: u32,
}

impl VadEngine {
    pub fn new(model_path: &Path, sample_rate: u32, silence_threshold_ms: u32) -> Result<Self, String> {
        let min_silence = silence_threshold_ms as f32 / 1000.0;
        let config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(model_path.to_string_lossy().into_owned()),
                min_silence_duration: min_silence,
                min_speech_duration: 0.25,
                threshold: 0.5,
                ..Default::default()
            },
            sample_rate: sample_rate as i32,
            num_threads: 1,
            provider: Some("cpu".into()),
            ..Default::default()
        };
        let detector = VoiceActivityDetector::create(&config, 60.0)
            .ok_or_else(|| "failed to create VAD".to_string())?;
        Ok(Self {
            detector,
            sample_rate,
        })
    }

    pub fn segment(&self, samples: &[f32], min_speech_ms: u32) -> Vec<Vec<f32>> {
        self.detector.accept_waveform(samples);
        self.detector.flush();
        let mut out = Vec::new();
        while let Some(seg) = self.detector.front() {
            let chunk = seg.samples().to_vec();
            if meets_min_duration(chunk.len(), self.sample_rate, min_speech_ms) {
                out.push(chunk);
            }
            self.detector.pop();
        }
        self.detector.reset();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_model_path_fails() {
        assert!(VadEngine::new(Path::new("/nonexistent/vad.onnx"), 16000, 800).is_err());
    }
}
