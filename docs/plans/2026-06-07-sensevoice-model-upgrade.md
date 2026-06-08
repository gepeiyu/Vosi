# SenseVoice 模型升级 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use tre:subagent-driven-development (recommended) or tre:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 Vosi ASR 从 `paraformer-zh-small` 升级为 SenseVoice INT8，支持中英日三语混说，测试期保留旧模型备份，验收通过后清理。

**Architecture:** 保持 sherpa-onnx `OfflineRecognizer` + 松手离线识别；`AsrEngine` 切换为 `OfflineSenseVoiceModelConfig`（`language=auto`, `use_itn=true`）；标点管线通过配置开关，Phase 1 golden 对比后决定去留；`ModelManager` 管理 `sense-voice/` 与 `paraformer-zh/` 双目录，新模型缺失时 fallback 旧模型。

**Tech Stack:** Rust / Tauri 2 / sherpa-onnx 1.13.2 / bash 下载脚本

**设计规格:** [`docs/specs/2026-06-07-vosi-model-upgrade-design.md`](../specs/2026-06-07-vosi-model-upgrade-design.md)

---

## 文件改动总览

| 文件 | 职责 |
|------|------|
| `models/manifest.json` | 模型清单、版本号、下载 URL |
| `scripts/download-models.sh` | 下载 SenseVoice；测试期保留 paraformer 下载 |
| `scripts/prepare-bundle-models.sh` | 不变（复制整个 `models/dev/`） |
| `src-tauri/src/config/types.rs` | 新增 `language`、`use_itn`、`punctuation_enabled` |
| `src-tauri/src/asr/paths.rs` | `resolve_sense_voice_paths()` |
| `src-tauri/src/asr/engine.rs` | SenseVoice 推理 |
| `src-tauri/src/asr/model_manager.rs` | 双模型路径、就绪检测、安装逻辑 |
| `src-tauri/src/pipeline/session.rs` | 可选标点步骤 |
| `src-tauri/src/lib.rs` | `dev_models_dir()` 检测逻辑 |
| `src-tauri/resources/hotwords-tech.txt` | 内置技术热词 |
| `src-tauri/src/post/hotword.rs` | 首次启动合并热词（新函数） |
| `tests/fixtures/audio/golden.json` | 15 条 golden 期望文本 |
| `tests/fixtures/audio/*.wav` | golden 音频（需录制） |
| `src-tauri/tests/asr_golden.rs` | 数据驱动 golden 测试 |
| `src-tauri/tests/asr_pipeline.rs` | 更新为 SenseVoice 路径 |
| `docs/guides/model-list.md` | 模型文档更新 |

---

### Task 1: 模型下载与 manifest

**Files:**
- Modify: `models/manifest.json`
- Modify: `scripts/download-models.sh`

- [ ] **Step 1: 更新 manifest.json**

将 `version` 改为 `0.2.0`，新增 SenseVoice 条目，保留 paraformer 作为 `legacy` 备份：

```json
{
  "version": "0.2.0",
  "default_asr": "sense-voice-int8",
  "models": [
    {
      "id": "sense-voice-int8",
      "dest": "models/dev/sense-voice",
      "github": "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2",
      "hf_mirror": "csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17",
      "files": ["model.int8.onnx", "tokens.txt"],
      "sha256": ""
    },
    {
      "id": "paraformer-zh-int8-legacy",
      "dest": "models/dev/paraformer-zh",
      "note": "测试期备份，验收后删除",
      "github": "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-paraformer-zh-small-2024-03-09.tar.bz2",
      "hf_mirror": "csukuangfj/sherpa-onnx-paraformer-zh-small-2024-03-09",
      "sha256": ""
    }
  ]
}
```

（保留现有 `silero-vad` 和 `punc-ct-transformer` 条目不变。）

- [ ] **Step 2: 在 download-models.sh 新增 download_sense_voice()**

在 `download_paraformer()` 之前插入：

```bash
download_sense_voice() {
  local dest="$DEST_ROOT/sense-voice"
  mkdir -p "$dest"

  if use_foreign_first || [[ "$MIRROR" == "hf-mirror" ]] || [[ "$MIRROR" == "auto" ]]; then
    if try_mirror hf \
      download_hf_file "csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17" "model.int8.onnx" "$dest/model.int8.onnx" \
      && download_hf_file "csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17" "tokens.txt" "$dest/tokens.txt"; then
      return 0
    fi
    [[ "$MIRROR" == "hf-mirror" ]] && return 1
  fi

  if [[ "$MIRROR" == "github" ]] || [[ "$MIRROR" == "auto" ]]; then
    try_mirror github download_github \
      "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2" \
      "$dest" && return 0
    [[ "$MIRROR" == "github" ]] && return 1
  fi

  return 1
}
```

将文件末尾调用改为：

```bash
download_sense_voice
download_paraformer   # 测试期保留备份
download_vad
download_punctuation
```

- [ ] **Step 3: 下载并验证**

```bash
export VOSI_PROXY=http://127.0.0.1:7890   # 如有代理
./scripts/download-models.sh
ls -lh models/dev/sense-voice/
# 预期: model.int8.onnx ~228M, tokens.txt ~309K
ls -lh models/dev/paraformer-zh/
# 预期: 旧模型仍在（备份）
```

- [ ] **Step 4: Commit**

```bash
git add models/manifest.json scripts/download-models.sh
git commit -m "chore: add SenseVoice model download and manifest v0.2.0"
```

---

### Task 2: 配置类型扩展

**Files:**
- Modify: `src-tauri/src/config/types.rs`
- Test: `src-tauri/src/config/mod.rs`（如有现有测试则扩展）

- [ ] **Step 1: 扩展 AsrConfig**

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AsrConfig {
    pub num_threads: u32,
    pub mode: String,
    pub model_variant: String,
    #[serde(default = "default_asr_language")]
    pub language: String,
    #[serde(default = "default_use_itn")]
    pub use_itn: bool,
    #[serde(default = "default_punctuation_enabled")]
    pub punctuation_enabled: bool,
}

fn default_asr_language() -> String {
    "auto".into()
}

fn default_use_itn() -> bool {
    true
}

fn default_punctuation_enabled() -> bool {
    true
}
```

更新 `Default for AppConfig`：

```rust
asr: AsrConfig {
    num_threads: 2,
    mode: "short".into(),
    model_variant: "sense-voice-int8".into(),
    language: default_asr_language(),
    use_itn: default_use_itn(),
    punctuation_enabled: default_punctuation_enabled(),
},
```

- [ ] **Step 2: 添加反序列化向后兼容测试**

在 `src-tauri/src/config/types.rs` 的 `#[cfg(test)]` 模块：

```rust
#[test]
fn asr_config_deserializes_without_new_fields() {
    let raw = r#"
    num_threads = 2
    mode = "short"
    model_variant = "paraformer-large-int8"
    "#;
    let table: toml::Table = toml::from_str(raw).unwrap();
    let cfg: AsrConfig = table.try_into().unwrap_or_else(|_| {
        toml::from_str(&format!("[asr]\n{raw}")).unwrap()
    });
    // 若项目用 serde 直接反序列化 AsrConfig，改用：
    let cfg: AsrConfig = toml::from_str(&format!("[asr]\n{raw}")).unwrap();
    assert_eq!(cfg.language, "auto");
    assert!(cfg.use_itn);
    assert!(cfg.punctuation_enabled);
}
```

（按项目实际 config 加载方式调整测试写法。）

- [ ] **Step 3: 运行测试**

```bash
cd src-tauri && cargo test config --lib
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/config/types.rs
git commit -m "feat: add SenseVoice ASR config fields (language, use_itn, punctuation)"
```

---

### Task 3: 模型路径解析

**Files:**
- Modify: `src-tauri/src/asr/paths.rs`
- Test: `src-tauri/src/asr/paths.rs`（内联测试）

- [ ] **Step 1: 新增 resolve_sense_voice_paths**

```rust
/// Resolve SenseVoice model.int8.onnx and tokens.txt.
pub fn resolve_sense_voice_paths(dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    if !dir.exists() {
        return Err(format!("sense-voice model dir not found: {}", dir.display()));
    }

    let search_roots: Vec<PathBuf> = if dir.is_dir() {
        let mut roots = vec![dir.to_path_buf()];
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    roots.push(entry.path());
                }
            }
        }
        roots
    } else {
        vec![dir.to_path_buf()]
    };

    for root in search_roots {
        if let Some(model) = first_existing(
            &root,
            &["model.int8.onnx", "model.onnx"],
        ) {
            let tokens = root.join("tokens.txt");
            if tokens.exists() {
                return Ok((model, tokens));
            }
        }
    }

    Err(format!(
        "could not find sense-voice model and tokens.txt under {}",
        dir.display()
    ))
}
```

- [ ] **Step 2: 添加单元测试**

```rust
#[test]
fn resolve_sense_voice_paths_finds_nested_files() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("pkg");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("model.int8.onnx"), b"x").unwrap();
    fs::write(nested.join("tokens.txt"), b"t").unwrap();

    let (model, tokens) = resolve_sense_voice_paths(dir.path()).unwrap();
    assert!(model.ends_with("model.int8.onnx"));
    assert!(tokens.ends_with("tokens.txt"));
}
```

- [ ] **Step 3: 运行测试**

```bash
cd src-tauri && cargo test resolve_sense_voice --lib
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/asr/paths.rs
git commit -m "feat: add sense-voice model path resolver"
```

---

### Task 4: AsrEngine 切换 SenseVoice

**Files:**
- Modify: `src-tauri/src/asr/engine.rs`

- [ ] **Step 1: 重写 AsrEngine**

```rust
use crate::asr::paths::resolve_sense_voice_paths;
use sherpa_onnx::{
    OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig,
};
use std::path::Path;

pub struct AsrEngine {
    recognizer: OfflineRecognizer,
}

pub struct AsrEngineOptions {
    pub language: String,
    pub use_itn: bool,
}

impl AsrEngine {
    pub fn new(
        sense_voice_dir: &Path,
        num_threads: i32,
        options: AsrEngineOptions,
    ) -> Result<Self, String> {
        let (model, tokens) = resolve_sense_voice_paths(sense_voice_dir)?;

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.sense_voice = OfflineSenseVoiceModelConfig {
            model: Some(model.to_string_lossy().into_owned()),
            language: Some(options.language),
            use_itn: options.use_itn,
        };
        config.model_config.tokens = Some(tokens.to_string_lossy().into_owned());
        config.model_config.num_threads = num_threads;
        config.model_config.provider = Some("cpu".into());

        let recognizer = OfflineRecognizer::create(&config)
            .ok_or_else(|| "failed to create OfflineRecognizer — check sense-voice model files".to_string())?;

        Ok(Self { recognizer })
    }

    pub fn transcribe(&self, samples: &[f32], sample_rate: u32) -> String {
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(sample_rate as i32, samples);
        self.recognizer.decode(&stream);
        stream
            .get_result()
            .map(|r| r.text)
            .unwrap_or_default()
    }
}
```

- [ ] **Step 2: 编译验证**

```bash
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
cd src-tauri && cargo build
```

Expected: 编译通过（调用方尚未更新，会有编译错误——Task 5 修复）

- [ ] **Step 3: Commit**（与 Task 5 一起提交，或先 stub 调用方）

---

### Task 5: ModelManager 双模型支持

**Files:**
- Modify: `src-tauri/src/asr/model_manager.rs`
- Modify: `src-tauri/src/lib.rs`（`dev_models_dir`）
- Modify: `src-tauri/src/pipeline/session.rs`

- [ ] **Step 1: 更新 ModelPaths 和就绪检测**

```rust
#[derive(Debug, Clone)]
pub struct ModelPaths {
    pub sense_voice_dir: PathBuf,
    pub paraformer_dir: PathBuf,  // 测试期备份
    pub vad_model: PathBuf,
    pub punc_dir: PathBuf,
}

impl ModelManager {
    pub fn resolve_paths(&self) -> ModelPaths {
        let base = self.models_dir();
        ModelPaths {
            sense_voice_dir: base.join("sense-voice"),
            paraformer_dir: base.join("paraformer-zh"),
            vad_model: base.join("vad/model.onnx"),
            punc_dir: base.join("punctuation"),
        }
    }

    pub fn sense_voice_ready(base: &Path) -> bool {
        let dir = base.join("sense-voice");
        ["model.int8.onnx", "model.onnx"]
            .iter()
            .any(|name| dir.join(name).exists())
            && dir.join("tokens.txt").exists()
    }

    pub fn active_asr_dir(paths: &ModelPaths) -> PathBuf {
        if Self::sense_voice_ready(paths.sense_voice_dir.parent().unwrap_or(Path::new(".")))
            || paths.sense_voice_dir.join("model.int8.onnx").exists()
        {
            paths.sense_voice_dir.clone()
        } else if Self::paraformer_ready(paths.paraformer_dir.parent().unwrap_or(Path::new(".")))
            || paths.paraformer_dir.join("model.int8.onnx").exists()
        {
            paths.paraformer_dir.clone()
        } else {
            paths.sense_voice_dir.clone()
        }
    }

    pub fn ensure_installed(&self, bundled: &Path, dev_fallback: Option<&Path>) -> std::io::Result<ModelPaths> {
        let dest = self.models_dir();
        if !Self::sense_voice_ready(&dest) && !Self::paraformer_ready(&dest) {
            std::fs::create_dir_all(&dest)?;
            copy_dir_all(bundled, &dest)?;
            if !Self::sense_voice_ready(&dest) {
                if let Some(dev) = dev_fallback {
                    copy_dir_all(dev, &dest)?;
                }
            }
        }
        Ok(self.resolve_paths())
    }
}
```

（`active_asr_dir` 逻辑简化为：优先 `sense-voice/`，不存在则 fallback `paraformer-zh/` 并记日志。）

- [ ] **Step 2: 更新 pipeline/session.rs**

```rust
use crate::asr::engine::{AsrEngine, AsrEngineOptions};

// try_new 内：
let paths = mgr.ensure_installed(bundled, dev_models).map_err(|e| e.to_string())?;
let asr_dir = ModelManager::active_asr_dir(&paths);
if asr_dir.ends_with("paraformer-zh") {
    logger.warn("sense-voice not found, falling back to paraformer-zh (legacy)");
}
let asr = AsrEngine::new(
    &asr_dir,
    config.asr.num_threads as i32,
    AsrEngineOptions {
        language: config.asr.language.clone(),
        use_itn: config.asr.use_itn,
    },
)?;

let punc = if config.asr.punctuation_enabled && paths.punc_dir.exists() {
    Some(PunctuationEngine::new(&paths.punc_dir, config.asr.num_threads as i32)?)
} else {
    None
};
```

`VoiceSession` 中 `punc: PunctuationEngine` 改为 `punc: Option<PunctuationEngine>`。

`finalize_recording` 中：

```rust
let punctuated = match punc {
    Some(engine) => engine.punctuate(&raw),
    None => raw.clone(),
};
```

- [ ] **Step 3: 更新 lib.rs dev_models_dir**

```rust
if crate::asr::ModelManager::sense_voice_ready(&dev)
    || crate::asr::ModelManager::paraformer_ready(&dev)
{
    return Some(dev);
}
```

- [ ] **Step 4: 编译 + 单元测试**

```bash
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
cd src-tauri && cargo test --lib && cargo build
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/asr/model_manager.rs src-tauri/src/asr/engine.rs \
        src-tauri/src/pipeline/session.rs src-tauri/src/lib.rs
git commit -m "feat: switch ASR pipeline to SenseVoice with legacy fallback"
```

---

### Task 6: Golden 测试集

**Files:**
- Create: `tests/fixtures/audio/golden.json`
- Create: `tests/fixtures/audio/*.wav`（15 条，需本地录制）
- Modify: `src-tauri/tests/asr_golden.rs`
- Modify: `tests/fixtures/audio/README.md`

- [ ] **Step 1: 创建 golden.json**

```json
[
  {
    "id": "zh_pure_1",
    "file": "zh_pure_1.wav",
    "category": "zh",
    "must_contain": ["开会", "排期"],
    "must_not_contain": []
  },
  {
    "id": "en_tech_react",
    "file": "en_tech_react.wav",
    "category": "en",
    "must_contain": ["React", "API"],
    "must_not_contain": ["瑞爱克特"]
  },
  {
    "id": "ja_mixed_impl",
    "file": "ja_mixed_impl.wav",
    "category": "ja",
    "must_contain": ["実装", "TypeScript"],
    "must_not_contain": []
  },
  {
    "id": "trilingual_1",
    "file": "trilingual_1.wav",
    "category": "mixed",
    "must_contain": ["API", "設計"],
    "must_not_contain": []
  },
  {
    "id": "pm_mvp",
    "file": "pm_mvp.wav",
    "category": "pm",
    "must_contain": ["MVP", "KPI"],
    "must_not_contain": []
  }
]
```

（完整 15 条按设计规格 §5.1 补全；上述为结构示例。）

- [ ] **Step 2: 重写 asr_golden.rs 为数据驱动**

```rust
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct GoldenCase {
    id: String,
    file: String,
    category: String,
    must_contain: Vec<String>,
    must_not_contain: Vec<String>,
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
    assert!(sense_voice_dir.exists(), "run ./scripts/download-models.sh");

    let engine = AsrEngine::new(
        &sense_voice_dir,
        2,
        AsrEngineOptions {
            language: "auto".into(),
            use_itn: true,
        },
    )
    .expect("asr engine");

    let wave = sherpa_onnx::Wave::read(wav.to_str().expect("utf-8")).expect("read wav");
    let raw = engine.transcribe(wave.samples(), wave.sample_rate() as u32);
    // 标点：Phase 1 对比时分别测 with/without punc
    raw
}

#[test]
#[ignore = "requires models and recorded fixtures"]
fn golden_all_cases() {
    for case in load_cases() {
        let text = transcribe_fixture(&case.file);
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
}
```

- [ ] **Step 3: 录制 golden WAV**

在安静环境用 macOS 录音或 REAPER/Audacity，导出 16kHz mono WAV，放入 `tests/fixtures/audio/`。

`tests/fixtures/audio/README.md` 记录录制话术与对应文件名。

- [ ] **Step 4: 运行 golden 测试**

```bash
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
cd src-tauri
cargo test --test asr_golden golden_all_cases -- --ignored --nocapture
```

Expected: 各项指标达到设计规格阈值

- [ ] **Step 5: Phase 1 标点对比**

分别运行 `punctuation_enabled=true` 和 `false`，统计 15 条样本标点正确率：

- ≥ 90%：`settings.toml` 默认 `punctuation_enabled = false`，后续 Task 10 删除标点模型
- < 90%：保持 `punctuation_enabled = true`

- [ ] **Step 6: Commit**

```bash
git add tests/fixtures/audio/ src-tauri/tests/asr_golden.rs
git commit -m "test: add SenseVoice golden fixtures and data-driven ASR tests"
```

---

### Task 7: 技术热词包

**Files:**
- Create: `src-tauri/resources/hotwords-tech.txt`
- Modify: `src-tauri/src/post/hotword.rs`
- Modify: `src-tauri/src/lib.rs` 或 `pipeline/session.rs`（首次启动合并）

- [ ] **Step 1: 创建热词文件**

```
React
TypeScript
JavaScript
Kubernetes
Docker
GitHub
API
pull request
merge request
npm
webpack
MVP
PRD
KPI
OKR
user story
sprint
backlog
roadmap
実装
設計
要件
バグ
デプロイ
レビュー
スプリント
```

（扩展至 50–100 条。）

- [ ] **Step 2: 添加 merge_builtin_hotwords 函数**

```rust
pub fn merge_builtin_hotwords(user_path: &Path, builtin_lines: &[&str]) -> std::io::Result<()> {
    let existing = std::fs::read_to_string(user_path).unwrap_or_default();
    let existing_set: std::collections::HashSet<String> = existing
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let mut additions = Vec::new();
    for word in builtin_lines {
        if !existing_set.contains(*word) {
            additions.push(*word);
        }
    }
    if additions.is_empty() {
        return Ok(());
    }
    use std::io::Write;
    if user_path.parent().map(|p| !p.exists()).unwrap_or(false) {
        std::fs::create_dir_all(user_path.parent().unwrap())?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(user_path)?;
    for word in additions {
        writeln!(file, "{word}")?;
    }
    Ok(())
}
```

在 `VoiceSession::try_new` 启动时调用，内置词从 `include_str!("../../resources/hotwords-tech.txt")` 读取。

- [ ] **Step 3: 添加单元测试**

```rust
#[test]
fn merge_builtin_hotwords_appends_without_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hotwords.txt");
    std::fs::write(&path, "React\n").unwrap();
    merge_builtin_hotwords(&path, &["React", "TypeScript"]).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content.matches("React").count(), 1);
    assert!(content.contains("TypeScript"));
}
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/resources/hotwords-tech.txt src-tauri/src/post/hotword.rs \
        src-tauri/src/pipeline/session.rs
git commit -m "feat: merge builtin tech hotwords on first session init"
```

---

### Task 8: 集成测试与本地验证

**Files:**
- Modify: `src-tauri/tests/asr_pipeline.rs`

- [ ] **Step 1: 更新 asr_pipeline.rs**

将 `paraformer_dir` 改为 `sense_voice_dir`，`AsrEngine::new` 传入 `AsrEngineOptions`。

- [ ] **Step 2: 端到端手动测试**

```bash
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
./scripts/download-models.sh
npm run tauri dev
```

测试清单：
- [ ] 中文纯句
- [ ] 「用 React 写个组件」
- [ ] 「这个功能的実装要用 TypeScript」
- [ ] 「这个 API の設計 review 一下」
- [ ] 「下个 sprint 对齐 MVP 和 KPI」

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/asr_pipeline.rs
git commit -m "test: update ASR pipeline integration test for SenseVoice"
```

---

### Task 9: 文档更新（测试期）

**Files:**
- Modify: `docs/guides/model-list.md`
- Modify: `docs/PROJECT-SUMMARY.md` §5

- [ ] **Step 1: 更新 model-list.md**

- 主 ASR 改为 SenseVoice INT8（~228MB）
- 注明测试期保留 paraformer-zh-small 备份
- 更新下载命令与目录结构

- [ ] **Step 2: 更新 PROJECT-SUMMARY.md 体积表**

| 角色 | 文件 | 大小 |
|------|------|------|
| ASR | `sense-voice/model.int8.onnx` | ~228MB |
| VAD | `vad/model.onnx` | ~2MB |
| 标点 | 可选 | ~280MB |

- [ ] **Step 3: Commit**

```bash
git add docs/guides/model-list.md docs/PROJECT-SUMMARY.md
git commit -m "docs: update model list for SenseVoice upgrade"
```

---

### Task 10: 验收后清理（Phase 4，golden 全部通过后执行）

**前置条件:** Task 6 golden 测试通过 + 人工验收通过

**Files:**
- Delete: `models/dev/paraformer-zh/`、`src-tauri/models/bundled/paraformer-zh/`
- Delete: `punctuation/`（仅当标点对比 ≥ 90% 无需保留时）
- Modify: `scripts/download-models.sh`（移除 `download_paraformer`）
- Modify: `models/manifest.json`（移除 legacy 条目）
- Modify: `src-tauri/src/asr/model_manager.rs`（移除 paraformer fallback）
- Modify: `src-tauri/src/asr/paths.rs`（可移除 `resolve_paraformer_paths` 若不再使用）

- [ ] **Step 1: 删除旧模型引用与下载逻辑**

- [ ] **Step 2: ModelManager 仅检测 sense-voice**

```rust
pub fn ensure_installed(...) {
    let dest = self.models_dir();
    if !Self::sense_voice_ready(&dest) {
        std::fs::create_dir_all(&dest)?;
        copy_dir_all(bundled, &dest)?;
        // dev_fallback 逻辑保留
    }
    // 验收后：删除旧 paraformer-zh 目录
    let legacy = dest.join("paraformer-zh");
    if legacy.exists() {
        let _ = std::fs::remove_dir_all(&legacy);
    }
    Ok(self.resolve_paths())
}
```

- [ ] **Step 3: 重新打包验证**

```bash
./scripts/prepare-bundle-models.sh
ls -lh src-tauri/models/bundled/
# 预期: 仅 sense-voice/ + vad/ (+ punctuation/ 若保留)
npm run tauri build
```

- [ ] **Step 4: Commit**

```bash
git commit -m "chore: remove legacy paraformer model after SenseVoice acceptance"
```

---

## 规格覆盖自检

| 规格要求 | 对应 Task |
|---------|----------|
| SenseVoice INT8 主选模型 | Task 1, 4 |
| language=auto 三语混说 | Task 2, 4 |
| 标点实测后决定去留 | Task 6 Step 5 |
| 测试期保留旧模型备份 | Task 1, 5 |
| 验收后清理旧模型 | Task 10 |
| 15 条 golden 测试 | Task 6 |
| 技术热词包 | Task 7 |
| 旧模型 fallback（测试期） | Task 5 |
| 文档更新 | Task 9 |

无 TBD / 占位符步骤。

---

## 执行选项

**Plan complete and saved to `docs/plans/2026-06-07-sensevoice-model-upgrade.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — 每个 Task 派发独立 subagent，任务间做 review，迭代快

**2. Inline Execution** — 在本会话按 Task 顺序直接实施，每 2–3 个 Task 设检查点

**Which approach?**
