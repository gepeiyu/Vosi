use tauri_app_lib::asr::engine::{AsrEngine, AsrEngineOptions};
use tauri_app_lib::asr::paths::resolve_sense_voice_paths;

#[test]
fn resolve_paths_helpers_work_without_models() {
    let dir = tempfile::tempdir().unwrap();
    assert!(resolve_sense_voice_paths(dir.path()).is_err());
}

#[test]
#[ignore = "requires downloaded models at models/dev/"]
fn asr_sense_voice_transcribes_fixture() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../models/dev");
    let sense_voice_dir = root.join("sense-voice");
    let wav = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/audio/short_greeting.wav");

    if !sense_voice_dir.join("model.int8.onnx").exists() {
        eprintln!("skip: sense-voice model.int8.onnx not downloaded");
        return;
    }
    if !wav.exists() {
        eprintln!("skip: fixture wav missing");
        return;
    }

    let engine = AsrEngine::new(
        &sense_voice_dir,
        2,
        AsrEngineOptions {
            language: "auto".into(),
            use_itn: true,
        },
    )
    .expect("asr engine");

    let wave = sherpa_onnx::Wave::read(wav.to_str().expect("wav path utf-8")).expect("read wav");
    let raw = engine.transcribe(wave.samples(), wave.sample_rate() as u32);
    assert!(!raw.trim().is_empty(), "expected non-empty transcription, got: {raw:?}");
    eprintln!("transcription: {raw}");
}
