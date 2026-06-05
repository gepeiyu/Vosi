use crate::asr::engine::AsrEngine;
use crate::asr::punctuation::PunctuationEngine;
use crate::asr::ModelManager;
use crate::audio::capture::AudioCapture;
use crate::config::AppConfig;
use crate::log::Logger;
use crate::post::hotword::HotwordReplacer;
use crate::post::pipeline::post_process;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

pub enum SessionState {
    Idle,
    Recording(AudioCapture),
}

pub struct VoiceSession {
    asr: AsrEngine,
    punc: PunctuationEngine,
    hotwords: HotwordReplacer,
    config: AppConfig,
    logger: Arc<Logger>,
    state: SessionState,
}

impl VoiceSession {
    pub fn try_new(
        config: AppConfig,
        models_root: PathBuf,
        bundled: &Path,
        logger: Arc<Logger>,
    ) -> Result<Self, String> {
        let mgr = ModelManager::new(models_root);
        let paths = mgr.ensure_installed(bundled).map_err(|e| e.to_string())?;
        let asr = AsrEngine::new(&paths.paraformer_dir, config.asr.num_threads as i32)?;
        let punc = PunctuationEngine::new(&paths.punc_dir, config.asr.num_threads as i32)?;
        let hotword_path = expand_tilde(&config.hotword.file);
        let hotwords = HotwordReplacer::from_file(&hotword_path)
            .unwrap_or_else(|_| HotwordReplacer::from_lines(vec![]));

        logger.info("voice session initialized");

        Ok(Self {
            asr,
            punc,
            hotwords,
            config,
            logger,
            state: SessionState::Idle,
        })
    }

    pub fn on_hotkey_press(&mut self) -> Result<(), String> {
        if matches!(self.state, SessionState::Idle) {
            let capture = AudioCapture::start(self.config.audio.sample_rate)?;
            self.logger.info("recording started");
            self.state = SessionState::Recording(capture);
        }
        Ok(())
    }

    pub fn on_hotkey_release(&mut self) -> Result<Option<String>, String> {
        let capture = match std::mem::replace(&mut self.state, SessionState::Idle) {
            SessionState::Recording(c) => c,
            SessionState::Idle => return Ok(None),
        };
        let (samples, sample_rate) = capture.stop();
        if !meets_min_duration(samples.len(), sample_rate, self.config.audio.min_speech_ms) {
            self.logger.info(&format!(
                "recording discarded sample_count={} reason=below_min_duration",
                samples.len()
            ));
            return Ok(None);
        }

        let sample_count = samples.len();
        let started = Instant::now();
        let result = finalize_recording(
            &self.asr,
            &self.punc,
            &self.hotwords,
            &self.config,
            samples,
            sample_rate,
        );
        self.logger.info(&format!(
            "inference_ms={} sample_count={} sample_rate={}",
            started.elapsed().as_millis(),
            sample_count,
            sample_rate
        ));
        result
    }
}

fn finalize_recording(
    asr: &AsrEngine,
    punc: &PunctuationEngine,
    hotwords: &HotwordReplacer,
    config: &AppConfig,
    samples: Vec<f32>,
    sample_rate: u32,
) -> Result<Option<String>, String> {
    let raw = asr.transcribe(&samples, sample_rate);
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let punctuated = punc.punctuate(&raw);
    let final_text = post_process(
        &punctuated,
        hotwords,
        config.hotword.enabled,
    );
    Ok(Some(final_text))
}

pub fn meets_min_duration(sample_count: usize, sample_rate: u32, min_speech_ms: u32) -> bool {
    sample_count >= min_samples_for_ms(sample_rate, min_speech_ms)
}

pub fn min_samples_for_ms(sample_rate: u32, ms: u32) -> usize {
    (sample_rate as usize * ms as usize) / 1000
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .map(|home| home.join(rest))
            .unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_audio_below_min_duration_is_rejected() {
        assert!(!meets_min_duration(
            min_samples_for_ms(16000, 299),
            16000,
            300
        ));
        assert!(meets_min_duration(
            min_samples_for_ms(16000, 300),
            16000,
            300
        ));
    }
}
