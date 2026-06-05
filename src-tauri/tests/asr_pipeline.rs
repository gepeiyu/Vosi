use tauri_app_lib::asr::engine::AsrEngine;
use tauri_app_lib::asr::paths::{resolve_paraformer_paths, resolve_punctuation_model};
use tauri_app_lib::asr::punctuation::PunctuationEngine;

#[test]
fn resolve_paths_helpers_work_without_models() {
    let dir = tempfile::tempdir().unwrap();
    assert!(resolve_paraformer_paths(dir.path()).is_err());
    assert!(resolve_punctuation_model(dir.path()).is_err());
}

#[test]
#[ignore = "requires downloaded models at models/dev/"]
fn asr_pipeline_produces_text_from_fixture() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../models/dev");
    let paraformer_dir = root.join("paraformer-zh");
    let punc_dir = root.join("punctuation");
    let wav = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/audio/short_greeting.wav");

    if !paraformer_dir.exists() {
        eprintln!("skip: models not downloaded");
        return;
    }

    let engine = AsrEngine::new(&paraformer_dir, 2).expect("asr engine");
    let punct = PunctuationEngine::new(&punc_dir, 1).expect("punctuation engine");

    if !wav.exists() {
        eprintln!("skip: fixture wav missing");
        return;
    }

    let wave = sherpa_onnx::Wave::read(&wav).expect("read wav");
    let raw = engine.transcribe(wave.samples(), wave.sample_rate() as u32);
    assert!(!raw.trim().is_empty(), "expected non-empty transcription");
    let final_text = punct.punctuate(&raw);
    assert!(!final_text.trim().is_empty());
}
