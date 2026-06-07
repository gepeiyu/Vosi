/// Linear resample mono f32 PCM to the target sample rate (e.g. mic 48000 → ASR 16000).
pub fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == 0 || to_rate == 0 || samples.is_empty() {
        return samples.to_vec();
    }
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let out_len = (samples.len() as u64 * to_rate as u64 / from_rate as u64) as usize;
    if out_len == 0 {
        return Vec::new();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let s0 = samples.get(idx).copied().unwrap_or(0.0);
        let s1 = samples.get(idx + 1).copied().unwrap_or(s0);
        out.push(s0 + frac * (s1 - s0));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_rate_is_noop() {
        let samples = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_linear(&samples, 16_000, 16_000), samples);
    }

    #[test]
    fn downsamples_48k_to_16k_by_three() {
        let samples: Vec<f32> = (0..48_000).map(|i| (i % 100) as f32 / 100.0).collect();
        let out = resample_linear(&samples, 48_000, 16_000);
        assert_eq!(out.len(), 16_000);
    }
}
