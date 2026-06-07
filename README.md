# Vosi

**Voice Operational Speech Input** — offline desktop voice input for Windows and macOS.

Hold a hotkey, speak, release — text appears in the focused app. 100% offline, zero Python runtime.

Powered by [FunASR](https://github.com/modelscope/FunASR) models, built on [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx).

![License](https://img.shields.io/badge/license-Apache--2.0-blue)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-lightgrey)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Release](https://img.shields.io/github/v/release/gepeiyu/Vosi?label=v0.1.0)

## Features (v0.1)

- Global hotkey hold-to-talk dictation
- FunASR Paraformer ASR + punctuation + ITN
- Custom hotword post-processing
- Configurable trigger key, inject method, ASR mode
- System tray + settings UI
- Privacy-safe logs (no recognized text stored)

## Quick start

```bash
npm install
./scripts/download-models.sh
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
npm run tauri dev
```

See [docs/guides/quick-start.md](docs/guides/quick-start.md) for permissions and troubleshooting.

## Documentation

| Doc | Description |
|-----|-------------|
| [**Project summary**](docs/PROJECT-SUMMARY.md) | Dev, models, permissions, release — start here |
| [Quick start (zh)](docs/guides/quick-start.md) | Install & first dictation |
| [Design spec](docs/specs/2026-06-05-vosi-v01-design.md) | v0.1 product & architecture |
| [Model list](docs/guides/model-list.md) | FunASR / sherpa-onnx models |
| [Manual test checklist](docs/guides/manual-test-checklist.md) | Pre-release QA |
| [Docs index](docs/README.md) | Full documentation map |

**Repository:** https://github.com/gepeiyu/Vosi · **Release:** [v0.1.0](https://github.com/gepeiyu/Vosi/releases/tag/v0.1.0)

## Project structure

```
src-tauri/     Rust backend (ASR, audio, hotkey, inject, tray)
src/           Settings UI (Vite + TypeScript)
models/        Model manifest & bundled weights
scripts/       Model download & release helpers
```

## License

Apache-2.0 — see [LICENSE](LICENSE).
