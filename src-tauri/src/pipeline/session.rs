use crate::asr::engine::{AsrEngine, AsrEngineOptions};
use crate::asr::ModelManager;
use crate::audio::capture::AudioCapture;
use crate::audio::level::rms_level;
use crate::audio::resample::resample_linear;
use crate::audio::vad::VadEngine;
use crate::config::AppConfig;
use crate::log::Logger;
use crate::post::hotword::{merge_builtin_hotwords, HotwordReplacer};
use crate::post::pipeline::post_process;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

const INFERENCE_TIMEOUT: Duration = Duration::from_secs(5);
/// Reject near-silent clips that make SenseVoice hallucinate ("Yeah.", "Okay.", etc.).
const MIN_RECORDING_RMS: f32 = 0.004;

pub enum SessionState {
    Idle,
    Recording(AudioCapture),
}

pub struct VoiceSession {
    asr: AsrEngine,
    hotwords: HotwordReplacer,
    config: AppConfig,
    logger: Arc<Logger>,
    vad: Option<VadEngine>,
    state: SessionState,
}

impl VoiceSession {
    pub fn try_new(
        config: AppConfig,
        models_root: PathBuf,
        bundled: &Path,
        dev_models: Option<&Path>,
        logger: Arc<Logger>,
    ) -> Result<Self, String> {
        let mgr = ModelManager::new(models_root);
        let paths = mgr
            .ensure_installed(bundled, dev_models)
            .map_err(|e| e.to_string())?;
        if !ModelManager::sense_voice_ready(&mgr.models_dir()) {
            return Err(
                "sense-voice model not found — run ./scripts/download-models.sh to install"
                    .into(),
            );
        }
        let asr = AsrEngine::new(
            &paths.sense_voice_dir,
            config.asr.num_threads as i32,
            AsrEngineOptions {
                language: config.asr.language.clone(),
                use_itn: config.asr.use_itn,
            },
        )?;
        const BUILTIN_HOTWORDS: &str = include_str!("../../resources/hotwords-tech.txt");
        let hotword_path = expand_tilde(&config.hotword.file);
        let builtin: Vec<&str> = BUILTIN_HOTWORDS
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        let _ = merge_builtin_hotwords(&hotword_path, &builtin);
        let hotwords = HotwordReplacer::from_file(&hotword_path)
            .unwrap_or_else(|_| HotwordReplacer::from_lines(vec![]));

        let vad = if config.asr.mode == "long" && paths.vad_model.exists() {
            Some(VadEngine::new(
                &paths.vad_model,
                16000,
                config.audio.silence_threshold_ms,
            )?)
        } else {
            None
        };

        logger.info("voice session initialized");

        Ok(Self {
            asr,
            hotwords,
            config,
            logger,
            vad,
            state: SessionState::Idle,
        })
    }

    pub fn on_hotkey_press(&mut self) -> Result<(), String> {
        self.on_hotkey_press_with_level(None)
    }

    pub fn on_hotkey_press_with_level(
        &mut self,
        level_tx: Option<mpsc::Sender<f32>>,
    ) -> Result<(), String> {
        if matches!(self.state, SessionState::Idle) {
            let capture =
                AudioCapture::start_with_level(self.config.audio.sample_rate, level_tx)?;
            self.logger.info("recording started");
            self.state = SessionState::Recording(capture);
        }
        Ok(())
    }

    /// Stop an in-progress recording without running ASR (e.g. accidental tap).
    pub fn cancel_recording(&mut self) {
        if let SessionState::Recording(capture) =
            std::mem::replace(&mut self.state, SessionState::Idle)
        {
            drop(capture);
            self.logger.info("recording cancelled");
        }
    }

    pub fn is_recording(&self) -> bool {
        matches!(self.state, SessionState::Recording(_))
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
        let duration_ms = (sample_count as u64 * 1000) / sample_rate as u64;
        let recording_rms = rms_level(&samples);
        self.logger.info(&format!(
            "recording duration_ms={duration_ms} rms={recording_rms:.4}"
        ));
        if recording_rms < MIN_RECORDING_RMS {
            self.logger.info("recording discarded: too quiet");
            return Ok(None);
        }

        let started = Instant::now();
        let target_rate = self.config.audio.sample_rate;
        let (samples, asr_rate) = if sample_rate != target_rate {
            self.logger.info(&format!(
                "resampling audio {sample_rate}Hz → {target_rate}Hz"
            ));
            (resample_linear(&samples, sample_rate, target_rate), target_rate)
        } else {
            (samples, sample_rate)
        };
        let result = finalize_recording(
            &self.asr,
            &self.hotwords,
            &self.config,
            self.vad.as_ref(),
            samples,
            asr_rate,
            &self.logger,
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

pub fn join_segments(parts: &[String]) -> String {
    parts.join("\n")
}

fn finalize_recording(
    asr: &AsrEngine,
    hotwords: &HotwordReplacer,
    config: &AppConfig,
    vad_engine: Option<&VadEngine>,
    samples: Vec<f32>,
    sample_rate: u32,
    logger: &Logger,
) -> Result<Option<String>, String> {
    let total_samples = samples.len();
    let segments: Vec<Vec<f32>> = if config.asr.mode == "long" {
        if let Some(vad) = vad_engine {
            let segs = vad.segment(&samples, config.audio.min_speech_ms);
            let kept: usize = segs.iter().map(|s| s.len()).sum();
            if segs.is_empty() || kept * 3 < total_samples {
                logger.info(&format!(
                    "vad kept {kept}/{total_samples} samples in {} segments — using full clip",
                    segs.len()
                ));
                vec![samples]
            } else {
                logger.info(&format!(
                    "vad segmented {total_samples} samples into {} parts ({kept} kept)",
                    segs.len()
                ));
                segs
            }
        } else {
            vec![samples]
        }
    } else {
        vec![samples]
    };

    if segments.is_empty() {
        return Ok(None);
    }

    let hotword_enabled = config.hotword.enabled;
    let (tx, rx) = mpsc::channel();

    // Scoped thread borrows engines (&T requires T: Sync). If this fails to compile,
    // inference runs synchronously on the pipeline thread and timeout is best-effort.
    let texts: Vec<String> = std::thread::scope(|s| -> Result<Vec<String>, String> {
        s.spawn(|| {
            let texts: Vec<String> = segments
                .iter()
                .filter_map(|seg| {
                    let raw = asr.transcribe(seg, sample_rate);
                    if raw.trim().is_empty() {
                        return None;
                    }
                    logger.info(&format!("asr raw: {}", truncate_log(&raw, 120)));
                    Some(post_process(&raw, hotwords, hotword_enabled))
                })
                .collect();
            let _ = tx.send(texts);
        });

        match rx.recv_timeout(INFERENCE_TIMEOUT) {
            Ok(texts) => Ok(texts),
            Err(mpsc::RecvTimeoutError::Timeout) => Err("inference timeout".into()),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err("inference thread disconnected".into())
            }
        }
    })?;

    if texts.is_empty() {
        Ok(None)
    } else {
        Ok(Some(join_segments(&texts)))
    }
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

fn truncate_log(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    format!("{}…", text.chars().take(max_chars).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_segments_inserts_newlines() {
        let parts = vec!["第一句".into(), "第二句".into()];
        assert_eq!(join_segments(&parts), "第一句\n第二句");
    }

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
