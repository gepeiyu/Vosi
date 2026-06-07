use crate::audio::level::rms_level;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, StreamConfig, SupportedStreamConfig};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

const PREFERRED_SAMPLE_RATE: u32 = 16_000;

pub struct AudioCapture {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    _stream: cpal::Stream,
}

impl AudioCapture {
    pub fn start(target_sample_rate: u32) -> Result<Self, String> {
        Self::start_with_level(target_sample_rate, None)
    }

    pub fn start_with_level(
        target_sample_rate: u32,
        level_tx: Option<Sender<f32>>,
    ) -> Result<Self, String> {
        let rate = if target_sample_rate > 0 {
            target_sample_rate
        } else {
            PREFERRED_SAMPLE_RATE
        };
        Self::open_input_stream(rate, level_tx)
    }

    /// Open the default mic briefly so macOS/Windows shows the permission prompt at launch.
    pub fn preflight_microphone() -> Result<(), String> {
        let capture = Self::open_input_stream(PREFERRED_SAMPLE_RATE, None)?;
        drop(capture);
        Ok(())
    }

    fn pick_input_config(device: &cpal::Device, target_rate: u32) -> Result<SupportedStreamConfig, String> {
        let configs: Vec<_> = device
            .supported_input_configs()
            .map_err(|e| e.to_string())?
            .collect();

        if let Some(range) = configs.iter().find(|range| {
            range.min_sample_rate().0 <= target_rate && range.max_sample_rate().0 >= target_rate
        }) {
            return Ok(range.with_sample_rate(cpal::SampleRate(target_rate)));
        }

        device
            .default_input_config()
            .map_err(|e| e.to_string())
    }

    fn open_input_stream(
        target_sample_rate: u32,
        level_tx: Option<Sender<f32>>,
    ) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no input device".to_string())?;
        let supported = Self::pick_input_config(&device, target_sample_rate)?;
        let sample_format = supported.sample_format();
        let sample_rate = supported.sample_rate().0;
        let config: StreamConfig = supported.into();

        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
        let buf = samples.clone();
        let since_last_level = Arc::new(AtomicUsize::new(0));

        let stream = match sample_format {
            SampleFormat::F32 => {
                let level_tx = level_tx.clone();
                let level_counter = since_last_level.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        append_samples(&buf, data.iter().copied());
                        maybe_emit_level(
                            &buf,
                            data.len(),
                            sample_rate,
                            &level_tx,
                            &level_counter,
                        );
                    },
                    stream_error,
                    None,
                )
            }
            SampleFormat::I16 => {
                let level_tx = level_tx.clone();
                let level_counter = since_last_level.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        append_samples(&buf, data.iter().map(|s| s.to_sample::<f32>()));
                        maybe_emit_level(
                            &buf,
                            data.len(),
                            sample_rate,
                            &level_tx,
                            &level_counter,
                        );
                    },
                    stream_error,
                    None,
                )
            }
            SampleFormat::U16 => {
                let level_tx = level_tx.clone();
                let level_counter = since_last_level.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        append_samples(&buf, data.iter().map(|s| s.to_sample::<f32>()));
                        maybe_emit_level(
                            &buf,
                            data.len(),
                            sample_rate,
                            &level_tx,
                            &level_counter,
                        );
                    },
                    stream_error,
                    None,
                )
            }
            other => return Err(format!("unsupported sample format: {other:?}")),
        }
        .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            samples,
            sample_rate,
            _stream: stream,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn stop(self) -> (Vec<f32>, u32) {
        drop(self._stream);
        let mut data = self.samples.lock().expect("audio buffer lock");
        (std::mem::take(&mut *data), self.sample_rate)
    }
}

fn should_emit_level(prev_count: usize, batch_len: usize, sample_rate: u32) -> bool {
    prev_count + batch_len >= sample_rate as usize / 20
}

fn maybe_emit_level(
    buf: &Arc<Mutex<Vec<f32>>>,
    batch_len: usize,
    sample_rate: u32,
    level_tx: &Option<Sender<f32>>,
    since_last_level: &AtomicUsize,
) {
    let Some(tx) = level_tx else {
        return;
    };
    let prev = since_last_level.fetch_add(batch_len, Ordering::Relaxed);
    if should_emit_level(prev, batch_len, sample_rate) {
        let guard = buf.lock().expect("audio buffer lock");
        let window = sample_rate as usize / 10;
        let tail = &guard[guard.len().saturating_sub(window)..];
        let _ = tx.send(rms_level(tail));
        since_last_level.store(0, Ordering::Relaxed);
    }
}

fn append_samples<I>(buf: &Arc<Mutex<Vec<f32>>>, iter: I)
where
    I: Iterator<Item = f32>,
{
    buf.lock().expect("audio buffer lock").extend(iter);
}

fn stream_error(err: cpal::StreamError) {
    eprintln!("audio stream error: {err}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_emit_level_after_fifty_ms_of_samples() {
        assert!(!should_emit_level(0, 799, 16_000));
        assert!(should_emit_level(0, 800, 16_000));
        assert!(should_emit_level(700, 100, 16_000));
    }

    #[test]
    #[ignore = "requires microphone hardware"]
    fn start_accepts_optional_level_sender() {
        let _ = AudioCapture::start_with_level(16_000, None);
    }
}
