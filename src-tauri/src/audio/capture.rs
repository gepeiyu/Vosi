use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, StreamConfig};
use std::sync::{Arc, Mutex};

pub struct AudioCapture {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    _stream: cpal::Stream,
}

impl AudioCapture {
    pub fn start(_target_sample_rate: u32) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no input device".to_string())?;
        let supported = device
            .default_input_config()
            .map_err(|e| e.to_string())?;
        let sample_format = supported.sample_format();
        let sample_rate = supported.sample_rate().0;
        let config: StreamConfig = supported.into();

        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
        let buf = samples.clone();
        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _| append_samples(&buf, data.iter().copied()),
                stream_error,
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    append_samples(&buf, data.iter().map(|s| s.to_sample::<f32>()))
                },
                stream_error,
                None,
            ),
            SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    append_samples(&buf, data.iter().map(|s| s.to_sample::<f32>()))
                },
                stream_error,
                None,
            ),
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
    #[test]
    fn module_links() {
        assert!(true);
    }
}
