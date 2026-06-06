use std::path::PathBuf;

fn vad_model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models/dev/vad/model.onnx")
}

#[test]
#[ignore = "requires downloaded VAD model"]
fn vad_splits_long_audio_into_segments() {
    let model = vad_model_path();
    if !model.exists() {
        return;
    }
    let engine = tauri_app_lib::audio::vad::VadEngine::new(&model, 16000, 800).unwrap();
    let wav = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/audio/short_greeting.wav"),
    )
    .expect("fixture");
    let samples: Vec<f32> = wav[44..]
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
        .collect();
    let triple: Vec<f32> = samples
        .iter()
        .chain(samples.iter())
        .chain(samples.iter())
        .cloned()
        .collect();
    let segments = engine.segment(&triple, 300);
    assert!(!segments.is_empty());
}
