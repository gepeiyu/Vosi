pub fn rms_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt().clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::rms_level;

    #[test]
    fn silence_is_zero() {
        assert_eq!(rms_level(&[0.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn full_scale_clamps_to_one() {
        assert_eq!(rms_level(&vec![1.0; 100]), 1.0);
    }

    #[test]
    fn half_amplitude_sine_rms() {
        let samples: Vec<f32> = (0..1000)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * i as f32 / 1000.0).sin())
            .collect();
        let level = rms_level(&samples);
        assert!(level > 0.0 && level <= 0.5);
    }
}
