use std::path::{Path, PathBuf};

use tauri_app_lib::asr::engine::AsrEngine;
use tauri_app_lib::asr::punctuation::PunctuationEngine;
use tauri_app_lib::post::hotword::HotwordReplacer;
use tauri_app_lib::post::pipeline::post_process;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/audio")
        .join(name)
}

fn models_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../models/dev")
}

fn transcribe_fixture(name: &str) -> String {
    let wav = fixture_path(name);
    assert!(wav.exists(), "missing fixture: {}", wav.display());

    let root = models_root();
    let paraformer_dir = root.join("paraformer-zh");
    let punc_dir = root.join("punctuation");
    assert!(
        paraformer_dir.exists(),
        "models not found — run ./scripts/download-models.sh"
    );

    let engine = AsrEngine::new(&paraformer_dir, 2).expect("asr engine");
    let punct = PunctuationEngine::new(&punc_dir, 1).expect("punctuation engine");
    let hotwords = HotwordReplacer::from_lines(vec![]);

    let wave = sherpa_onnx::Wave::read(&wav).expect("read wav");
    let raw = engine.transcribe(wave.samples(), wave.sample_rate() as u32);
    let punctuated = punct.punctuate(&raw);
    post_process(&punctuated, &hotwords, false)
}

#[test]
#[ignore = "requires models and recorded fixtures"]
fn golden_short_greeting() {
    let text = transcribe_fixture("short_greeting.wav");
    assert!(text.contains("你好"), "got: {text}");
}

#[test]
#[ignore = "requires models and recorded fixtures"]
fn golden_number_amount() {
    let text = transcribe_fixture("number_amount.wav");
    assert!(text.contains("123") || text.contains("一百二十三"), "got: {text}");
}

#[test]
#[ignore = "requires models and recorded fixtures"]
fn golden_date_sentence() {
    let text = transcribe_fixture("date_sentence.wav");
    assert!(text.contains("2026") || text.contains("六月"), "got: {text}");
}

#[test]
#[ignore = "requires models and recorded fixtures"]
fn golden_mixed_en_cn() {
    let text = transcribe_fixture("mixed_en_cn.wav");
    assert!(
        text.to_lowercase().contains("chrome") || text.contains("浏览器"),
        "got: {text}"
    );
}

#[test]
#[ignore = "requires models and recorded fixtures"]
fn golden_long_paragraph_non_empty() {
    let text = transcribe_fixture("long_paragraph.wav");
    assert!(text.trim().len() > 4, "got: {text}");
}
