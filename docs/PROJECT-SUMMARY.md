# Vosi 项目总结

> 最后更新：2026-06-08  
> 仓库：https://github.com/gepeiyu/Vosi  
> 许可证：Apache-2.0

本文档汇总 Vosi 从 v0.1 实现、开源发布到 v0.1.1 polish 的**必要运维与开发信息**，作为单一入口；细节规格见各专项文档。

---

## 1. 项目是什么

**Vosi**（Voice Operational Speech Input）是全离线桌面语音输入工具，面向 Windows / macOS：

- **按住热键** → 说话 → **松手** → 识别文字注入当前焦点应用
- 100% 离线，用户侧零 Python 依赖
- 推理：FunASR **SenseVoice** INT8（多语 `language=auto`）+ 可选 CT-Transformer 标点 + ITN，经 [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) Rust crate 1.13.x
- 壳层：Tauri 2 + Rust 后端 + Vite/TS 设置界面

---

## 2. 开源与版本

| 项 | 说明 |
|----|------|
| GitHub | https://github.com/gepeiyu/Vosi |
| 首个公开 Release | [v0.1.0](https://github.com/gepeiyu/Vosi/releases/tag/v0.1.0)（2026-06-06，源码发布） |
| 当前开发分支 | `feat/v0.1.1-polish`（polish + SenseVoice 升级） |
| **应用版本号** | **`0.1.1`**（`tauri.conf.json` / `Cargo.toml` / `package.json`） |
| **本地打包产物** | `Vosi_0.1.1_x64.dmg`（须与版本号一致；全量 `npm run tauri build` 生成） |

### 里程碑

| 日期 | 事件 |
|------|------|
| 2026-06-05 | v0.1 设计规格 + 16 项实现任务完成 |
| 2026-06-06 | 模型下载链路、ASR 集成测试、macOS E2E 语音输入验证 |
| 2026-06-06 | 推送 `main`、打 tag `v0.1.0`、GitHub 正式开源 |
| 2026-06-06–07 | v0.1.1 polish：热键修复、浮窗、品牌图标、三平台 Release workflow |
| 2026-06-07–08 | SenseVoice INT8 升级、权限引导、Task 10 清理 legacy paraformer；人工验收通过 |

---

## 3. 架构概览

```
用户按住热键
    → hotkey/（macOS: CGEventTap；Windows: 全局监听）
    → audio/capture（cpal 录音）
    → [长句模式] audio/vad（Silero 分段）
    → asr/engine（SenseVoice 离线识别）
    → asr/punctuation（CT-Transformer 标点）
    → post/（热词替换 → ITN）
    → inject/（enigo 键盘模拟 / arboard 剪贴板）
    → 焦点应用收到文字
```

| 模块 | 路径 | 职责 |
|------|------|------|
| 配置 | `src-tauri/src/config/` | `settings.toml` 读写 |
| 管线 | `src-tauri/src/pipeline/` | Hold-to-Talk 状态机 |
| ASR | `src-tauri/src/asr/` | 模型路径、引擎、标点 |
| 热键 | `src-tauri/src/hotkey/` | 平台热键监听 |
| 注入 | `src-tauri/src/inject/` | macOS / Windows 文本注入 |
| 浮窗 | `src-tauri/src/overlay/` | 录音胶囊 UI |
| 托盘 | `src-tauri/src/app/tray.rs` | 三态图标 |
| 日志 | `src-tauri/src/log/` | 隐私安全日志（不含识别文本） |
| 设置 UI | `index.html`, `src/main.ts` | Tauri IPC 配置 |

---

## 4. 环境要求

- macOS 12+ 或 Windows 10+
- Node.js 20+
- Rust stable
- 磁盘：模型约 **510 MB**（ASR 228MB + 标点 281MB + VAD 2MB）
- 首次编译：sherpa-onnx 静态库约 18MB（缓存于 `SHERPA_ONNX_ARCHIVE_DIR`）

---

## 5. 模型下载（必读）

### 5.1 格式要求

**ASR 与标点必须使用 sherpa-onnx 预打包 ONNX**（含 `vocab_size` / `tokens` metadata）。

魔搭 ModelScope 上的 FunASR 原生 `model_quant.onnx` **与 sherpa-onnx 不兼容**，会报错如：

```
'vocab_size' does not exist in the metadata
'tokens' does not exist in the metadata
```

### 5.2 推荐命令

```bash
# 有本地代理（推荐）
export VOSI_PROXY=http://127.0.0.1:7890
./scripts/download-models.sh

# 无代理：HuggingFace 国内镜像
VOSI_MODEL_MIRROR=hf-mirror ./scripts/download-models.sh
```

### 5.3 模型清单

| 角色 | 文件 | 大小 | 来源 |
|------|------|------|------|
| ASR | `sense-voice/model.int8.onnx` + `tokens.txt` | ~228MB | [csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17](https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17) |
| 标点 | `punctuation/model.onnx` | ~281MB | [csukuangfj/sherpa-onnx-punct-ct-transformer](https://huggingface.co/csukuangfj/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12) |
| VAD | `vad/model.onnx` | ~629KB | GitHub `silero_vad.onnx` |

安装包体积：不含标点约 **~230MB**（ASR + VAD）；含标点约 **~510MB**。

安装目录：`models/dev/`（开发）→ `prepare-bundle-models.sh` → `src-tauri/models/bundled/`（Release 打包）。

详见 [guides/model-list.md](guides/model-list.md)。

---

## 6. 开发与运行

```bash
git clone https://github.com/gepeiyu/Vosi.git
cd Vosi && npm install

export VOSI_PROXY=http://127.0.0.1:7890          # 国内可选
./scripts/download-models.sh
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"

npm run tauri dev
```

### 6.1 Debug 模型安装

Debug 构建会自动将 `models/dev/` 复制到用户数据目录（若 bundled 为空）：

- macOS 模型：`~/Library/Application Support/vosi/models/`
- 配置：`~/Library/Application Support/vosi/settings.toml`（`dirs` crate；部分文档亦写作 `~/.config/vosi/`）

### 6.2 运行测试

```bash
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
cd src-tauri

cargo test --lib                                    # 单元测试（含 ModelManager）
cargo test --test asr_pipeline -- --ignored         # ASR 管线（需 models/dev）
cargo test --test asr_golden -- --ignored           # Golden 音频（需 fixture）
```

Golden 占位音频：`tests/fixtures/audio/short_greeting.wav`（来自 sherpa 官方 test_wavs/0.wav）。

---

## 7. 权限（macOS）

| 权限 | 用途 |
|------|------|
| 麦克风 | 录音 |
| 辅助功能 | CGEventTap 热键 + enigo 文本注入 |

### 开发模式 vs 安装包

| 模式 | 系统设置中显示 |
|------|----------------|
| `tauri dev` | **tauri-app** 可执行文件路径（设置页顶部横幅会显示完整路径） |
| Release `.dmg` | **Vosi.app** |

授权步骤：

1. 托盘 → **设置**，按横幅路径添加 **辅助功能**
2. **按住触发键**说话，在麦克风弹窗中允许

---

## 8. 默认热键与交互

| 平台 | 默认触发键 | 配置值 |
|------|-----------|--------|
| macOS | 空格右侧 **Command ⌘** | `RightCommand` |
| Windows | 空格右侧 **Alt** | `RightAlt` |

### v0.1.1 交互规则

- **轻点（< 300 ms）**：无反应（不录音、不显示浮窗）
- **按住 ≥ 300 ms**：开始录音 + 浮窗；松手后识别并注入
- macOS **仅监听配置侧 Command**（默认右 ⌘）；左 ⌘ 不触发，避免 ⌘+C 误唤起

可在设置界面修改触发键。

---

## 9. 打包与发布

### 9.1 本地构建

```bash
./scripts/download-models.sh
./scripts/prepare-bundle-models.sh
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
npm run tauri build
```

| 平台 | 产物 |
|------|------|
| macOS | `src-tauri/target/release/bundle/dmg/*.dmg` |
| Windows | `src-tauri/target/release/bundle/nsis/*.exe` |

**原则**：在目标系统上构建；Mac 无法直接出 Windows 包。

### 9.2 GitHub Actions Release

推送 tag `v*` 触发 `.github/workflows/release.yml`：

| Job | Runner | 产物 |
|-----|--------|------|
| macOS arm64 | `macos-latest` | `.dmg` |
| macOS x64 | `macos-15-intel` | `.dmg` |
| Windows x64 | `windows-latest` | `.exe` |

发版前检查：[guides/manual-test-checklist.md](guides/manual-test-checklist.md)

---

## 10. 日志与隐私

- 路径（macOS）：`~/Library/Application Support/vosi/logs/vosi.log`
- **不记录识别文本**，仅元数据（`inference_ms`、`sample_count`、错误码等）
- 1 MB 轮转

---

## 11. 已知限制

| 项 | 说明 |
|----|------|
| 魔搭 ONNX 不兼容 | 须用 sherpa 预打包模型（见 §5） |
| macOS 浮窗焦点 | 录音浮窗可能短暂激活 Vosi，原输入框或失焦（后续 NSPanel 优化） |
| SenseVoice 无 runtime 热词 | 采用后处理文本替换 + 内置技术热词包 |
| Golden 15 条 WAV | 基础设施就绪，音频待 TTS/录制（不阻塞发版） |
| 标点对比 | 暂缓；当前默认保留 CT-Transformer 标点 |
| 开机自启 | UI 有选项，平台注册未实现 → [follow-ups/2026-06-06-start-on-boot-platform-registration.md](follow-ups/2026-06-06-start-on-boot-platform-registration.md) |
| 模型不入 Git | `models/dev/`、`src-tauri/models/bundled/` 约 510MB，构建前脚本下载 |

---

## 12. 验证记录

| 验证项 | 日期 | 结果 |
|--------|------|------|
| 单元测试 `cargo test --lib` | 2026-06-06 | 11/11 PASS |
| ASR 集成 `asr_pipeline` | 2026-06-06 | PASS |
| Golden `golden_short_greeting` | 2026-06-06 | PASS |
| macOS E2E 按住说话 + 文本注入 | 2026-06-06 | 用户确认正常 |
| GitHub 推送 + v0.1.0 Release | 2026-06-06 | 完成 |
| CI clippy + tests（polish 分支） | 2026-06-07 | PASS |
| SenseVoice 升级 + `cargo test --lib` | 2026-06-08 | 24/24 PASS |
| macOS E2E SenseVoice dictation | 2026-06-08 | 用户确认正常 |
| 本地 Release DMG（v0.1.1） | 2026-06-08 | bump 后须重打 `Vosi_0.1.1_*.dmg` |

---

## 13. 文档索引

| 文档 | 说明 |
|------|------|
| **本文** | 项目总结（运维/开发入口） |
| [guides/quick-start.md](guides/quick-start.md) | 快速上手 |
| [guides/model-list.md](guides/model-list.md) | 模型下载详解 |
| [guides/manual-test-checklist.md](guides/manual-test-checklist.md) | 发版前手动测试 |
| [specs/2026-06-05-vosi-v01-design.md](specs/2026-06-05-vosi-v01-design.md) | v0.1 产品设计 |
| [specs/2026-06-06-vosi-v01-polish-design.md](specs/2026-06-06-vosi-v01-polish-design.md) | v0.1.1 polish 设计 |
| [plans/2026-06-05-vosi-v01.md](plans/2026-06-05-vosi-v01.md) | v0.1 实现计划 |
| [plans/2026-06-06-vosi-v01-polish.md](plans/2026-06-06-vosi-v01-polish.md) | v0.1.1 实现计划 |
| [logs/2026-06-05-vosi-v01-execution-log.md](logs/2026-06-05-vosi-v01-execution-log.md) | v0.1 执行日志 |
| [logs/2026-06-06-vosi-v01-polish-execution-log.md](logs/2026-06-06-vosi-v01-polish-execution-log.md) | polish 执行日志 |
| [logs/2026-06-07-v01-polish-wrap-up.md](logs/2026-06-07-v01-polish-wrap-up.md) | v0.1.1 收尾（品牌/打包/Release） |
| [specs/2026-06-07-vosi-model-upgrade-design.md](specs/2026-06-07-vosi-model-upgrade-design.md) | SenseVoice 升级设计 |
| [logs/2026-06-07-sensevoice-model-upgrade-execution-log.md](logs/2026-06-07-sensevoice-model-upgrade-execution-log.md) | SenseVoice 升级执行日志 |
| [follow-ups/2026-06-06-start-on-boot-platform-registration.md](follow-ups/2026-06-06-start-on-boot-platform-registration.md) | 开机自启待办 |

---

## 14. 关键脚本

| 脚本 | 用途 |
|------|------|
| `scripts/download-models.sh` | 下载 ASR/VAD/标点到 `models/dev/` |
| `scripts/prepare-bundle-models.sh` | 复制模型到 `src-tauri/models/bundled/` |
| `scripts/generate-logo.py` | 生成 App / 托盘图标 PNG |

环境变量：

| 变量 | 说明 |
|------|------|
| `VOSI_PROXY` | HTTP 代理，如 `http://127.0.0.1:7890` |
| `VOSI_MODEL_MIRROR` | `auto` / `github` / `hf-mirror` / `modelscope` |
| `SHERPA_ONNX_ARCHIVE_DIR` | sherpa-onnx 静态库缓存目录 |
