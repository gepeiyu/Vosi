use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri_app_lib::asr::engine::{AsrEngine, AsrEngineOptions};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GoldenCase {
    id: String,
    file: String,
    category: String,
    #[serde(default)]
    script: Option<String>,
    must_contain: Vec<String>,
    #[serde(default)]
    must_not_contain: Vec<String>,
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/audio")
        .join(name)
}

fn models_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../models/dev")
}

fn load_cases() -> Vec<GoldenCase> {
    let path = fixture_path("golden.json");
    let raw = fs::read_to_string(&path).expect("golden.json");
    serde_json::from_str(&raw).expect("parse golden.json")
}

fn transcribe_fixture(name: &str) -> String {
    let wav = fixture_path(name);
    assert!(wav.exists(), "missing fixture: {}", wav.display());

    let root = models_root();
    let sense_voice_dir = root.join("sense-voice");
    assert!(
        sense_voice_dir.exists(),
        "models not found — run ./scripts/download-models.sh"
    );

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
    engine.transcribe(wave.samples(), wave.sample_rate() as u32)
}

#[test]
#[ignore = "requires models and recorded fixtures"]
fn golden_all_cases() {
    let cases = load_cases();
    let mut ran = 0;

    for case in &cases {
        let wav = fixture_path(&case.file);
        if !wav.exists() {
            println!("skip [{}]: missing {}", case.id, wav.display());
            continue;
        }

        let text = transcribe_fixture(&case.file);
        println!("[{}] {}", case.id, text);
        ran += 1;

        for needle in &case.must_contain {
            assert!(
                text.contains(needle),
                "[{}] expected {:?} in {:?}",
                case.id,
                needle,
                text
            );
        }
        for needle in &case.must_not_contain {
            assert!(
                !text.contains(needle),
                "[{}] must not contain {:?} in {:?}",
                case.id,
                needle,
                text
            );
        }
    }

    assert!(ran > 0, "no golden WAV fixtures found — record audio per README.md");
}

/// Phase 1 punctuation comparison: run the same 15 golden cases with
/// `punctuation_enabled=true` vs SenseVoice ITN-only output, then compare
/// punctuation accuracy. Threshold ≥ 90% → default `punctuation_enabled=false`
/// and remove CT-Transformer in Task 10; otherwise keep punctuation pipeline.
#[test]
#[ignore = "Phase 1 manual comparison — enable after golden WAV fixtures exist"]
fn golden_punctuation_comparison() {
    // TODO: transcribe each case twice (with/without PunctuationEngine),
    // score punctuation against expected scripts in golden.json, log summary.
}
