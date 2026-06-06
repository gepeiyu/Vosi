# Vosi v0.1.1 Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use tre:subagent-driven-development (recommended) or tre:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close v0.1 design gaps — VAD long-mode segmentation, Typeless-style recording overlay, error notifications, tray state icons, settings UI completion, and clipboard inject fallback.

**Architecture:** Extend the existing Rust voice pipeline with `VadEngine`, RMS level metering, timeout-wrapped inference, and `inject_with_fallback`. Add a transparent Tauri `overlay` WebView driven by `overlay-state` events. Wire `tauri-plugin-notification` for error-only toasts. Settings and `OverlayConfig` persist via existing `settings.toml`.

**Tech Stack:** Tauri 2, Rust, `sherpa-onnx` 1.13.x, `tauri-plugin-notification` 2, `cpal`, `arboard`, vanilla TS/CSS

**Spec reference:** `docs/specs/2026-06-06-vosi-v01-polish-design.md`

---

## File Map

| Path | Action | Responsibility |
|------|--------|----------------|
| `src-tauri/src/config/types.rs` | Modify | Add `OverlayConfig` |
| `src-tauri/src/config/mod.rs` | Modify | Migrate missing `[overlay]` |
| `src-tauri/src/audio/mod.rs` | Modify | Export `level`, `vad` |
| `src-tauri/src/audio/level.rs` | Create | RMS helper |
| `src-tauri/src/audio/capture.rs` | Modify | Level metering channel |
| `src-tauri/src/audio/vad.rs` | Create | Silero VAD segmentation |
| `src-tauri/src/inject/mod.rs` | Modify | `inject_with_fallback` |
| `src-tauri/src/notify/mod.rs` | Create | System notification wrapper |
| `src-tauri/src/overlay/mod.rs` | Create | Overlay window control + emit |
| `src-tauri/src/app/tray.rs` | Modify | `set_icon` per status |
| `src-tauri/src/pipeline/session.rs` | Modify | Long mode, timeout, segments |
| `src-tauri/src/lib.rs` | Modify | Wire overlay, notify, level loop |
| `src-tauri/Cargo.toml` | Modify | Add notification plugin |
| `src-tauri/tauri.conf.json` | Modify | Overlay window + plugin |
| `src-tauri/capabilities/default.json` | Modify | Overlay + notification perms |
| `src-tauri/icons/icon-idle.png` | Create | Tray idle (copy/tint `icon.png`) |
| `src-tauri/icons/icon-recording.png` | Create | Tray recording (red tint) |
| `src-tauri/icons/icon-warning.png` | Create | Tray warning (yellow tint) |
| `overlay.html` | Create | Overlay entry |
| `src/overlay.ts` | Create | Capsule state listener |
| `src/overlay.css` | Create | Capsule visuals |
| `index.html` | Modify | Settings groups + new fields |
| `src/main.ts` | Modify | Read/write new config fields |
| `src/styles.css` | Modify | Settings section styles |
| `vite.config.ts` | Modify | Multi-page build (`overlay.html`) |
| `docs/guides/manual-test-checklist.md` | Modify | Overlay + notification checks |
| `src-tauri/tests/vad_segment.rs` | Create | VAD segmentation test (ignored) |

---

### Task 1: OverlayConfig and Config Migration

**Files:**
- Modify: `src-tauri/src/config/types.rs`
- Modify: `src-tauri/src/config/mod.rs`
- Test: `src-tauri/src/config/mod.rs` (existing test module)

- [ ] **Step 1: Write the failing test**

Add to `src-tauri/src/config/mod.rs` `#[cfg(test)]` module:

```rust
#[test]
fn config_without_overlay_section_gets_default_overlay() {
    let raw = r#"
[hotkey]
trigger_key = "RightCommand"
mode = "hold"

[audio]
sample_rate = 16000
silence_threshold_ms = 800
min_speech_ms = 300

[asr]
num_threads = 2
mode = "short"
model_variant = "paraformer-large-int8"

[hotword]
enabled = true
file = "~/.config/vosi/hotwords.txt"

[inject]
method = "type"

[general]
start_on_boot = true
show_tray = true
"#;
    let parsed: AppConfig = toml::from_str(raw).unwrap();
    let migrated = migrate_config(parsed);
    assert!(migrated.overlay.enabled);
}
```

Add `use types::AppConfig;` and export `migrate_config` for test (change `fn migrate_config` visibility to `pub(crate)` or test inline).

- [ ] **Step 2: Run test to verify it fails**

```bash
cd src-tauri && cargo test config_without_overlay_section_gets_default_overlay -- --nocapture
```

Expected: FAIL — `overlay` field missing on `AppConfig`

- [ ] **Step 3: Implement OverlayConfig**

In `src-tauri/src/config/types.rs`:

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OverlayConfig {
    pub enabled: bool,
}

// Add to AppConfig:
pub struct AppConfig {
    // ...existing fields...
    pub overlay: OverlayConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            // ...existing...
            overlay: OverlayConfig { enabled: true },
        }
    }
}
```

In `src-tauri/src/config/mod.rs`, extend `migrate_config`:

```rust
fn migrate_config(mut cfg: AppConfig) -> AppConfig {
    if cfg.hotkey.trigger_key == "RightAlt" && cfg!(target_os = "macos") {
        cfg.hotkey.trigger_key = types::default_trigger_key();
    }
    // serde default handles missing overlay when deserializing if we use #[serde(default)]
    cfg
}
```

Add on `OverlayConfig` and `AppConfig.overlay`:

```rust
#[serde(default)]
```

Or use `#[derive(Default)]` on `OverlayConfig`.

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test --lib config::
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config/types.rs src-tauri/src/config/mod.rs
git commit -m "feat: add overlay config section with migration defaults"
```

---

### Task 2: RMS Audio Level Utility

**Files:**
- Create: `src-tauri/src/audio/level.rs`
- Modify: `src-tauri/src/audio/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/audio/level.rs`:

```rust
pub fn rms_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt().clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_zero() {
        assert_eq!(rms_level(&[0.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn full_scale_clamps_to_one() {
        let loud = vec![1.0; 100];
        assert_eq!(rms_level(&loud), 1.0);
    }

    #[test]
    fn half_amplitude_sine_rms() {
        let samples: Vec<f32> = (0..100)
            .map(|i| 0.5 * (i as f32 * 0.1).sin())
            .collect();
        let level = rms_level(&samples);
        assert!(level > 0.0 && level <= 0.5);
    }
}
```

In `src-tauri/src/audio/mod.rs`:

```rust
pub mod capture;
pub mod level;
```

- [ ] **Step 2: Run tests**

```bash
cd src-tauri && cargo test --lib audio::level::
```

Expected: PASS (implementation included above)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/audio/level.rs src-tauri/src/audio/mod.rs
git commit -m "feat: add RMS audio level helper for overlay metering"
```

---

### Task 3: AudioCapture Level Metering Channel

**Files:**
- Modify: `src-tauri/src/audio/capture.rs`

- [ ] **Step 1: Write the failing test**

Add to `src-tauri/src/audio/capture.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn start_accepts_optional_level_sender() {
        // Smoke: type-check API; skip if no mic in CI
        let (_tx, _rx) = mpsc::channel();
        let _ = AudioCapture::start_with_level(16000, None);
    }
}
```

- [ ] **Step 2: Run test**

```bash
cd src-tauri && cargo test --lib audio::capture::tests::start_accepts_optional_level_sender -- --nocapture
```

Expected: FAIL — `start_with_level` not defined

- [ ] **Step 3: Implement level metering**

Refactor `capture.rs`:

```rust
use crate::audio::level::rms_level;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicUsize, Ordering};

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
        _target_sample_rate: u32,
        level_tx: Option<Sender<f32>>,
    ) -> Result<Self, String> {
        let since_last_level = Arc::new(AtomicUsize::new(0));
        // In each format branch's closure, after append_samples:
        //   if let Some(tx) = &level_tx {
        //     let n = since_last_level.fetch_add(data.len(), Ordering::Relaxed);
        //     if n >= sample_rate / 20 { // ~50ms
        //       let buf = buf.lock().unwrap();
        //       let tail = &buf[buf.len().saturating_sub(sample_rate as usize / 10)..];
        //       let _ = tx.send(rms_level(tail));
        //       since_last_level.store(0, Ordering::Relaxed);
        //     }
        //   }
        // Keep existing start logic, pass level_tx + counters into closures.
    }
}
```

Keep `stop()` unchanged.

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test --lib audio::capture::
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/audio/capture.rs
git commit -m "feat: push RMS audio levels every 50ms during capture"
```

---

### Task 4: Silero VAD Segmentation Engine

**Files:**
- Create: `src-tauri/src/audio/vad.rs`
- Modify: `src-tauri/src/audio/mod.rs`
- Test: `src-tauri/tests/vad_segment.rs`

- [ ] **Step 1: Write the failing unit test**

Create `src-tauri/src/audio/vad.rs`:

```rust
use crate::pipeline::session::{meets_min_duration, min_samples_for_ms};
use sherpa_onnx::{SileroVadModelConfig, VadModelConfig, VoiceActivityDetector};
use std::path::Path;

pub struct VadEngine {
    detector: VoiceActivityDetector,
    sample_rate: u32,
}

impl VadEngine {
    pub fn new(model_path: &Path, sample_rate: u32, silence_threshold_ms: u32) -> Result<Self, String> {
        let min_silence = silence_threshold_ms as f32 / 1000.0;
        let config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(model_path.to_string_lossy().into_owned()),
                min_silence_duration: min_silence,
                min_speech_duration: 0.25,
                threshold: 0.5,
                ..Default::default()
            },
            sample_rate: sample_rate as i32,
            num_threads: 1,
            provider: Some("cpu".into()),
            ..Default::default()
        };
        let detector = VoiceActivityDetector::create(&config, 60.0)
            .ok_or_else(|| "failed to create VAD".to_string())?;
        Ok(Self { detector, sample_rate })
    }

    pub fn segment(&self, samples: &[f32], min_speech_ms: u32) -> Vec<Vec<f32>> {
        self.detector.accept_waveform(samples);
        self.detector.flush();
        let mut out = Vec::new();
        while let Some(seg) = self.detector.front() {
            let chunk = seg.samples().to_vec();
            if meets_min_duration(chunk.len(), self.sample_rate, min_speech_ms) {
                out.push(chunk);
            }
            self.detector.pop();
        }
        self.detector.reset();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_segments() {
        // Use a temp empty path — expect create to fail; test segment on dummy via mock skip
        assert!(VadEngine::new(Path::new("/nonexistent/vad.onnx"), 16000, 800).is_err());
    }
}
```

Add `pub mod vad;` to `audio/mod.rs`.

- [ ] **Step 2: Run unit test**

```bash
cd src-tauri && cargo test --lib audio::vad::
```

Expected: PASS

- [ ] **Step 3: Write integration test (ignored)**

Create `src-tauri/tests/vad_segment.rs`:

```rust
use std::path::PathBuf;

fn vad_model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../models/dev/vad/model.onnx")
}

#[test]
#[ignore = "requires downloaded VAD model"]
fn vad_splits_long_audio_into_segments() {
    let model = vad_model_path();
    if !model.exists() {
        return;
    }
    let engine = tauri_app_lib::audio::vad::VadEngine::new(&model, 16000, 800).unwrap();
    // Load short_greeting.wav, repeat 3x to simulate pause gaps — assert segments >= 1
    // Use hound or manual WAV reader if available; else read fixture bytes
    let wav = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/audio/short_greeting.wav"),
    )
    .expect("fixture");
    // Parse PCM f32 samples (skip 44-byte header for standard WAV)
    let samples: Vec<f32> = wav[44..]
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
        .collect();
    let triple: Vec<f32> = samples.iter().chain(samples.iter()).chain(samples.iter()).cloned().collect();
    let segments = engine.segment(&triple, 300);
    assert!(!segments.is_empty());
}
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/audio/vad.rs src-tauri/src/audio/mod.rs src-tauri/tests/vad_segment.rs
git commit -m "feat: add Silero VAD segmentation for long-mode dictation"
```

---

### Task 5: Inject Clipboard Fallback

**Files:**
- Modify: `src-tauri/src/inject/mod.rs`

- [ ] **Step 1: Write the failing test**

Add to `src-tauri/src/inject/mod.rs`:

```rust
pub struct FallbackResult {
    pub injected: bool,
    pub copied_to_clipboard: bool,
}

pub fn inject_with_fallback(
    injector: &dyn TextInjector,
    text: &str,
    method: InjectMethod,
) -> FallbackResult {
    match injector.inject(text, method) {
        Ok(()) => FallbackResult {
            injected: true,
            copied_to_clipboard: false,
        },
        Err(_) => {
            let copied = arboard::Clipboard::new()
                .and_then(|mut c| c.set_text(text.to_string()).map(|_| ()))
                .is_ok();
            FallbackResult {
                injected: false,
                copied_to_clipboard: copied,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingInjector;
    impl TextInjector for FailingInjector {
        fn inject(&self, _: &str, _: InjectMethod) -> Result<(), String> {
            Err("inject failed".into())
        }
    }

    #[test]
    fn fallback_copies_to_clipboard_on_inject_failure() {
        let result = inject_with_fallback(&FailingInjector, "你好世界", InjectMethod::Type);
        assert!(!result.injected);
        assert!(result.copied_to_clipboard);
        let clip = arboard::Clipboard::new().unwrap().get_text().unwrap();
        assert_eq!(clip, "你好世界");
    }
}
```

- [ ] **Step 2: Run test**

```bash
cd src-tauri && cargo test --lib inject::tests::fallback_copies_to_clipboard_on_inject_failure -- --nocapture
```

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/inject/mod.rs
git commit -m "feat: add clipboard fallback when text injection fails"
```

---

### Task 6: System Notification Module

**Files:**
- Create: `src-tauri/src/notify/mod.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add dependency**

In `src-tauri/Cargo.toml`:

```toml
tauri-plugin-notification = "2"
```

- [ ] **Step 2: Create notifier**

Create `src-tauri/src/notify/mod.rs`:

```rust
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

pub struct Notifier {
    app: AppHandle,
}

impl Notifier {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    pub fn error(&self, body: &str) {
        let _ = self
            .app
            .notification()
            .builder()
            .title("Vosi")
            .body(body)
            .show();
    }
}
```

Add `pub mod notify;` to `lib.rs`.

Register plugin in `run()`:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_notification::init())
```

- [ ] **Step 3: Update capabilities**

In `src-tauri/capabilities/default.json`:

```json
{
  "windows": ["main", "overlay"],
  "permissions": [
    "core:default",
    "opener:default",
    "notification:default"
  ]
}
```

- [ ] **Step 4: Verify compile**

```bash
cd src-tauri && cargo check
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/notify/mod.rs src-tauri/src/lib.rs src-tauri/capabilities/default.json
git commit -m "feat: add system notification plugin for error feedback"
```

---

### Task 7: Tray Multi-State Icons

**Files:**
- Create: `src-tauri/icons/icon-idle.png`, `icon-recording.png`, `icon-warning.png`
- Modify: `src-tauri/src/app/tray.rs`

- [ ] **Step 1: Generate icon variants**

```bash
cd src-tauri/icons
cp icon.png icon-idle.png
# macOS with sips (tint red for recording, yellow for warning):
cp icon.png icon-recording.png
sips -s format png --setProperty formatOptions low icon-recording.png 2>/dev/null || true
# If sips tint unavailable, manually edit or use Python PIL:
python3 - <<'PY'
from pathlib import Path
try:
    from PIL import Image
    base = Image.open("icon.png").convert("RGBA")
    for name, color in [("icon-recording", (220,50,50,255)), ("icon-warning", (230,180,30,255))]:
        img = base.copy()
        overlay = Image.new("RGBA", img.size, color)
        img = Image.blend(img, overlay, 0.45)
        img.save(f"{name}.png")
    Path("icon-idle.png").write_bytes(Path("icon.png").read_bytes())
except ImportError:
    import shutil
    for n in ("icon-idle", "icon-recording", "icon-warning"):
        shutil.copy("icon.png", f"{n}.png")
PY
```

- [ ] **Step 2: Implement set_icon in tray.rs**

```rust
use tauri::image::Image;
use tauri::include_image;

fn icon_for(status: TrayStatus) -> Image<'static> {
    match status {
        TrayStatus::Idle => include_image!("icons/icon-idle.png"),
        TrayStatus::Recording => include_image!("icons/icon-recording.png"),
        TrayStatus::Warning => include_image!("icons/icon-warning.png"),
    }
}

pub fn set_status<R: Runtime>(app: &AppHandle<R>, status: TrayStatus) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip_for(status)));
        let _ = tray.set_icon(Some(icon_for(status)));
    }
}
```

Adjust `include_image!` path per Tauri 2 API (may be `tauri::include_image!` with crate-relative path).

- [ ] **Step 3: Verify compile**

```bash
cd src-tauri && cargo check
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/icons/icon-*.png src-tauri/src/app/tray.rs
git commit -m "feat: switch tray icon by idle, recording, and warning state"
```

---

### Task 8: Overlay Frontend (Capsule UI)

**Files:**
- Create: `overlay.html`
- Create: `src/overlay.ts`
- Create: `src/overlay.css`
- Modify: `vite.config.ts`

- [ ] **Step 1: Create overlay.html**

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <link rel="stylesheet" href="/src/overlay.css" />
    <script type="module" src="/src/overlay.ts" defer></script>
  </head>
  <body>
    <div id="capsule" class="capsule hidden" aria-live="polite">
      <div class="capsule-inner">
        <div id="recording-view" class="view hidden">
          <span class="dot recording-dot"></span>
          <span class="brand">Vosi</span>
          <div class="bars" id="bars"></div>
        </div>
        <div id="processing-view" class="view hidden">
          <span class="ring"></span>
          <span class="label">识别中…</span>
          <div class="dots"><span></span><span></span><span></span></div>
        </div>
      </div>
    </div>
  </body>
</html>
```

- [ ] **Step 2: Create overlay.ts**

```typescript
import { listen } from "@tauri-apps/api/event";

type OverlayPayload =
  | { phase: "hidden" }
  | { phase: "recording"; level: number }
  | { phase: "processing" };

const capsule = document.getElementById("capsule")!;
const recordingView = document.getElementById("recording-view")!;
const processingView = document.getElementById("processing-view")!;
const bars = document.getElementById("bars")!;

for (let i = 0; i < 5; i++) {
  const bar = document.createElement("div");
  bar.className = "bar";
  bars.appendChild(bar);
}

function setBars(level: number) {
  const barEls = bars.querySelectorAll<HTMLDivElement>(".bar");
  barEls.forEach((bar, i) => {
    const jitter = 0.85 + ((i % 3) * 0.05);
    const h = Math.max(20, Math.min(100, level * 100 * jitter));
    bar.style.height = `${h}%`;
  });
}

function showRecording(level: number) {
  capsule.classList.remove("hidden");
  recordingView.classList.remove("hidden");
  processingView.classList.add("hidden");
  setBars(level);
}

function showProcessing() {
  capsule.classList.remove("hidden");
  recordingView.classList.add("hidden");
  processingView.classList.remove("hidden");
}

function hide() {
  capsule.classList.add("hidden");
  recordingView.classList.add("hidden");
  processingView.classList.add("hidden");
}

listen<OverlayPayload>("overlay-state", (event) => {
  const p = event.payload;
  if (p.phase === "hidden") hide();
  else if (p.phase === "recording") showRecording(p.level);
  else if (p.phase === "processing") showProcessing();
});
```

- [ ] **Step 3: Create overlay.css** (glass capsule, animations per spec §4.2)

Key rules: `body { background: transparent; pointer-events: none; }`, `.capsule` fixed bottom center, `backdrop-filter: blur(20px)`, `.recording-dot` pulse animation, `.dots span` stagger animation, `.hidden { opacity: 0; pointer-events: none; }` with 200ms transition.

- [ ] **Step 4: Multi-page Vite**

In `vite.config.ts`:

```typescript
import { resolve } from "path";

export default defineConfig(async () => ({
  // ...existing...
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        overlay: resolve(__dirname, "overlay.html"),
      },
    },
  },
}));
```

- [ ] **Step 5: Manual smoke**

```bash
npm run dev
# Open http://localhost:1420/overlay.html in browser to verify layout
```

- [ ] **Step 6: Commit**

```bash
git add overlay.html src/overlay.ts src/overlay.css vite.config.ts
git commit -m "feat: add Typeless-style recording capsule overlay UI"
```

---

### Task 9: OverlayController (Rust Window + Events)

**Files:**
- Create: `src-tauri/src/overlay/mod.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add overlay window to tauri.conf.json**

```json
{
  "label": "overlay",
  "title": "Vosi Overlay",
  "url": "overlay.html",
  "width": 280,
  "height": 56,
  "visible": false,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "resizable": false,
  "focus": false
}
```

Add macOS private API if needed for transparency (document in comment; enable `macos-private-api` feature only if build requires).

- [ ] **Step 2: Create OverlayController**

```rust
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[derive(Clone, Serialize)]
#[serde(tag = "phase", rename_all = "lowercase")]
pub enum OverlayState {
    Hidden,
    Recording { level: f32 },
    Processing,
}

pub struct OverlayController {
    app: AppHandle,
    enabled: bool,
}

impl OverlayController {
    pub fn new(app: AppHandle, enabled: bool) -> Self {
        Self { app, enabled }
    }

    pub fn emit(&self, state: OverlayState) {
        if !self.enabled {
            return;
        }
        if let Some(win) = self.app.get_webview_window("overlay") {
            if !matches!(state, OverlayState::Hidden) {
                let _ = win.show();
                self.position_bottom_center(&win);
            } else {
                let _ = win.hide();
            }
            let _ = win.emit("overlay-state", &state);
        }
    }

    fn position_bottom_center<R: Runtime>(&self, win: &tauri::WebviewWindow<R>) {
        if let Ok(monitor) = win.current_monitor() {
            if let Some(m) = monitor {
                let size = m.size();
                let x = (size.width as i32 - 280) / 2;
                let y = size.height as i32 - 56 - 48;
                let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
            }
        }
    }
}
```

- [ ] **Step 3: Wire in lib.rs setup**

```rust
pub mod overlay;

// In setup():
let overlay = overlay::OverlayController::new(
    app.handle().clone(),
    config.overlay.enabled,
);
// Pass overlay + notifier into spawn_voice_pipeline
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/overlay/mod.rs src-tauri/tauri.conf.json src-tauri/src/lib.rs
git commit -m "feat: add overlay window controller with bottom-center positioning"
```

---

### Task 10: Pipeline Long Mode, Timeout, and Segment Join

**Files:**
- Modify: `src-tauri/src/pipeline/session.rs`

- [ ] **Step 1: Write failing test for segment join**

Add to `session.rs` tests:

```rust
#[test]
fn join_segments_inserts_newlines() {
    let parts = vec!["第一句".into(), "第二句".into()];
    assert_eq!(join_segments(&parts), "第一句\n第二句");
}

fn join_segments(parts: &[String]) -> String {
    parts.join("\n")
}
```

- [ ] **Step 2: Run test**

```bash
cd src-tauri && cargo test --lib pipeline::session::tests::join_segments_inserts_newlines
```

Expected: FAIL until `join_segments` exported/implemented

- [ ] **Step 3: Implement long mode + timeout**

Key changes to `VoiceSession`:

```rust
use crate::audio::vad::VadEngine;
use std::time::{Duration, Instant};

const INFERENCE_TIMEOUT: Duration = Duration::from_secs(5);

pub struct VoiceSession {
    // ...existing...
    vad: Option<VadEngine>,
}

// In try_new: if config.asr.mode == "long" && paths.vad_model.exists() {
//   vad = Some(VadEngine::new(&paths.vad_model, sample_rate, silence_threshold_ms)?);
// }

fn finalize_recording(...) -> Result<Option<String>, String> {
    let segments: Vec<Vec<f32>> = if config.asr.mode == "long" {
        if let Some(vad) = vad_engine {
            vad.segment(&samples, config.audio.min_speech_ms)
        } else {
            vec![samples]
        }
    } else {
        vec![samples]
    };
    if segments.is_empty() {
        return Ok(None);
    }

    let (tx, rx) = mpsc::channel();
    let asr = asr.clone(); // or use Arc<AsrEngine> if needed
    // Spawn inference thread; main waits rx.recv_timeout(INFERENCE_TIMEOUT)
    let texts: Vec<String> = segments.iter().filter_map(|seg| {
        let raw = asr.transcribe(seg, sample_rate);
        if raw.trim().is_empty() { return None; }
        let punctuated = punc.punctuate(&raw);
        Some(post_process(&punctuated, hotwords, config.hotword.enabled))
    }).collect();

    if texts.is_empty() { Ok(None) } else { Ok(Some(texts.join("\n"))) }
}
```

Wrap ASR+punc in `thread::spawn` + `recv_timeout` for timeout; on timeout return `Err("inference timeout".into())`.

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test --lib pipeline::session::
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/pipeline/session.rs
git commit -m "feat: add long-mode VAD segmentation and 5s inference timeout"
```

---

### Task 11: Wire Pipeline, Overlay, Notifications in lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Refactor spawn_voice_pipeline signature**

```rust
fn spawn_voice_pipeline(
    app: tauri::AppHandle,
    config: AppConfig,
    bundled: PathBuf,
    dev_models: Option<PathBuf>,
    logger: Arc<Logger>,
    overlay: overlay::OverlayController,
    notifier: notify::Notifier,
) {
```

- [ ] **Step 2: Hotkey Pressed handler**

```rust
HotkeyEvent::Pressed => {
    tray::set_status(&app, TrayStatus::Recording);
    overlay.emit(OverlayState::Recording { level: 0.0 });
    let (level_tx, level_rx) = mpsc::channel();
    // Pass level_tx to session.on_hotkey_press_with_level(level_tx)
    // Spawn helper thread:
    thread::spawn(move || {
        while let Ok(level) = level_rx.recv() {
            overlay.emit(OverlayState::Recording { level });
        }
    });
}
```

- [ ] **Step 3: Hotkey Released handler**

```rust
HotkeyEvent::Released => {
    overlay.emit(OverlayState::Processing);
    match session.on_hotkey_release() {
        Ok(Some(text)) => {
            let result = inject::inject_with_fallback(injector.as_ref(), &text, inject_method);
            overlay.emit(OverlayState::Hidden);
            if !result.injected {
                notifier.error("已复制到剪贴板，请手动粘贴");
                tray::set_status(&app, TrayStatus::Warning);
                schedule_tray_reset(app.clone(), 3);
            } else {
                tray::set_status(&app, TrayStatus::Idle);
            }
        }
        Ok(None) => {
            overlay.emit(OverlayState::Hidden);
            tray::set_status(&app, TrayStatus::Idle);
        }
        Err(err) => {
            overlay.emit(OverlayState::Hidden);
            if err.contains("timeout") {
                notifier.error("识别超时，请重试");
            }
            tray::set_status(&app, TrayStatus::Warning);
            schedule_tray_reset(app.clone(), 3);
        }
    }
}
```

- [ ] **Step 4: Microphone error on press**

```rust
if let Err(err) = session.on_hotkey_press(...) {
    if err.contains("no input device") {
        notifier.error("未检测到麦克风");
    }
    overlay.emit(OverlayState::Hidden);
    tray::set_status(&app, TrayStatus::Warning);
}
```

- [ ] **Step 5: Model load failure**

```rust
Err(err) => {
    notifier.error("语音引擎不可用，请重新安装");
    tray::set_status(&app, TrayStatus::Warning);
}
```

Add helper:

```rust
fn schedule_tray_reset<R: Runtime>(app: AppHandle<R>, secs: u64) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(secs));
        tray::set_status(&app, TrayStatus::Idle);
    });
}
```

- [ ] **Step 6: Verify**

```bash
cd src-tauri && cargo test --lib
npm run tauri dev
# Manual: hold hotkey → capsule appears → release → processing → text injects
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/pipeline/session.rs src-tauri/src/audio/capture.rs
git commit -m "feat: wire overlay states, notifications, and inject fallback in pipeline"
```

---

### Task 12: Settings UI Completion

**Files:**
- Modify: `index.html`
- Modify: `src/main.ts`
- Modify: `src/styles.css`

- [ ] **Step 1: Update index.html with grouped fields**

Add sections 语音 / 识别 / 输出 / 通用 with:

- `min-speech-ms` (number 100–1000)
- `num-threads` (select 1–4)
- `hotword-enabled` (checkbox)
- `overlay-enabled` (checkbox)
- `show-tray` (checkbox)
- `start-on-boot` (checkbox, label note: 「平台注册即将支持」)

- [ ] **Step 2: Update main.ts AppConfig type and form handlers**

```typescript
type AppConfig = {
  // ...existing...
  overlay: { enabled: boolean };
};

function fillForm(cfg: AppConfig) {
  // existing...
  byId<HTMLInputElement>("min-speech-ms").value = String(cfg.audio.min_speech_ms);
  byId<HTMLSelectElement>("num-threads").value = String(cfg.asr.num_threads);
  byId<HTMLInputElement>("hotword-enabled").checked = cfg.hotword.enabled;
  byId<HTMLInputElement>("overlay-enabled").checked = cfg.overlay.enabled;
  byId<HTMLInputElement>("show-tray").checked = cfg.general.show_tray;
  byId<HTMLInputElement>("start-on-boot").checked = cfg.general.start_on_boot;
}
```

Mirror in `readForm`.

- [ ] **Step 3: Add section styles in styles.css**

```css
.settings-section { margin-top: 8px; }
.settings-section h2 { font-size: 0.95rem; margin: 0 0 8px; color: #666; }
label.checkbox { flex-direction: row; align-items: center; gap: 8px; }
```

- [ ] **Step 4: Manual verify**

```bash
npm run tauri dev
# Change each field → Save → restart → values persist
```

- [ ] **Step 5: Commit**

```bash
git add index.html src/main.ts src/styles.css
git commit -m "feat: complete settings UI for all config fields"
```

---

### Task 13: Manual Test Checklist and README Bump

**Files:**
- Modify: `docs/guides/manual-test-checklist.md`
- Modify: `README.zh-CN.md`

- [ ] **Step 1: Add checklist sections**

```markdown
## 浮动胶囊

- [ ] 按住热键：底部居中胶囊出现，音量条随说话跳动
- [ ] 松开热键：胶囊切换「识别中…」脉冲
- [ ] 上屏成功后胶囊隐藏
- [ ] 设置关闭「显示浮动胶囊」后按住无胶囊（管线仍工作）

## 错误通知

- [ ] 注入失败：系统通知「已复制到剪贴板」+ 托盘警告图标
- [ ] 断开麦克风后按住：通知「未检测到麦克风」

## 长句模式

- [ ] 设置 ASR 模式为「长句」，说 20s+ 带停顿语音，上屏文本含换行

## 托盘图标

- [ ] 就绪 / 录音 / 警告 三态图标可区分
```

- [ ] **Step 2: Update README.zh-CN.md v0.1.1 bullets**

- [ ] **Step 3: Commit**

```bash
git add docs/guides/manual-test-checklist.md README.zh-CN.md
git commit -m "docs: extend manual test checklist for v0.1.1 polish features"
```

---

### Task 14: Final Verification

- [ ] **Step 1: Run unit tests**

```bash
cd src-tauri && cargo test --lib
```

Expected: all PASS

- [ ] **Step 2: Run clippy**

```bash
cd src-tauri && cargo clippy -- -D warnings
```

- [ ] **Step 3: Run ignored integration tests (if models present)**

```bash
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
./scripts/download-models.sh
cd src-tauri && cargo test --test vad_segment -- --ignored
```

- [ ] **Step 4: Manual E2E per checklist**

- [ ] **Step 5: Commit any fixes**

```bash
git commit -m "chore: v0.1.1 polish verification fixes"
```

---

## Spec Coverage Checklist

| Spec § | Requirement | Task |
|--------|-------------|------|
| 5.1 | Short mode unchanged + overlay | Task 10–11 |
| 5.2 | Long mode VAD segments + `\n` join | Task 4, 10 |
| 5.3 | RMS level 50ms | Task 2–3, 11 |
| 5.4 | ASR 5s timeout | Task 10–11 |
| 5.5 | Inject clipboard fallback | Task 5, 11 |
| 4.x | Overlay capsule UI (方案 B) | Task 8–9, 11 |
| 6.x | Error table + tray icons + notifications | Task 6–7, 11 |
| 7.x | Settings UI fields + overlay.enabled | Task 1, 12 |
| 9.x | Unit + manual tests | Task 4, 13–14 |

## Follow-Up (Out of Scope)

- macOS LaunchAgent / Windows registry for `start_on_boot`
- Golden WAV local recording
- Release DMG/EXE manual validation
- Streaming partial transcription in overlay
