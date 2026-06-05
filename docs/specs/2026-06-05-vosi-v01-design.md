# Vosi v0.1 设计规格

> **Voice Operational Speech Input** — 基于 FunASR 模型生态的桌面离线语音输入工具
>
> 日期：2026-06-05 | 状态：已评审待实施

## 1. 背景与目标

### 1.1 产品定位

Vosi 是一款基于 FunASR 模型生态驱动、全离线、双平台专属的桌面智能语音输入工具，专注 Windows / macOS 端，主打「纯本地离线、轻量化低负担、中文场景深度适配」。

**官方 Slogan**：让桌面语音输入与操控更简单、更高效

### 1.2 v0.1 范围（方案 B）

**纳入：**

- 全局热键按住说话、松手上屏
- FunASR Paraformer + VAD + 标点 + ITN（数字/时间归一）
- 自定义热词库（后处理文本替换）
- VAD 静音截断
- 短句/长文模式切换
- 可配置：热键、灵敏度、VAD 阈值
- Windows EXE / macOS DMG 开箱即用，内置 INT8 模型
- 100% 离线，无网络请求

**明确排除（v0.2+）：**

- 语音快捷指令操控
- 场景润色（公文/聊天模式）
- 20+ 方言模型
- 会议说话人区分
- 创作润色模式
- 零日志隐私模式

### 1.3 核心约束

- **运行时零 Python**：用户侧不依赖 Python 环境
- **双平台**：Windows + macOS（universal binary）
- **对标 FunASR 开源范式**：Apache-2.0 许可、模型清单、文档分层

---

## 2. 技术选型

| 层级 | 选型 | 理由 |
|------|------|------|
| 桌面壳 | Tauri 2 (Rust + WebView) | 常驻后台内存低（目标 <80MB 空载），单二进制分发 |
| ASR 引擎 | sherpa-onnx (C API via Rust FFI) | 零 Python、跨平台预编译；CapsWriter 已验证 FunASR 模型链路 |
| 模型 | FunASR ONNX 预导出包（INT8） | Paraformer-large + FSMN-VAD + Punc-CT-Transformer |
| 音频采集 | `cpal` (Rust) | 跨平台麦克风，16kHz mono PCM |
| 全局热键 | `rdev` / 平台原生插件 | 按住说话、松手上屏 |
| 文本注入 | 平台原生 API | macOS: CGEvent；Windows: SendInput |
| 设置 UI | Tauri WebView | 托盘菜单 + 独立设置窗口 |
| 打包 | Tauri bundler | Win: NSIS 单文件 EXE；macOS: DMG |

### 2.1 与 FunASR 的关系

- **模型层**：使用 FunASR ModelScope 发布的 ONNX 模型（Paraformer、VAD、标点）
- **运行时层**：sherpa-onnx 加载并推理这些模型（非 FunASR Python/C++ SDK）
- **构建时**：开发者从 ModelScope 下载预导出 ONNX，用户无需 Python
- **品牌叙事**：「Vosi — powered by FunASR models, built on sherpa-onnx runtime」

---

## 3. 系统架构

```
┌─────────────────────────────────────┐
│  Vosi 单进程应用 (Tauri 2)           │
│  ├─ 系统托盘                         │
│  ├─ 全局热键监听                     │
│  ├─ 音频采集 (cpal)                  │
│  ├─ VAD 静音截断                     │
│  ├─ sherpa-onnx ASR Pipeline        │
│  ├─ 后处理: ITN + 热词替换           │
│  ├─ 文本注入引擎                     │
│  ├─ 设置 UI                          │
│  └─ 模型管理器                       │
└─────────────────────────────────────┘
         ↓ 加载
┌─────────────────────────────────────┐
│  内置模型 (~300MB INT8)              │
│  ├─ Paraformer-large INT8           │
│  ├─ FSMN-VAD INT8                   │
│  └─ Punc-CT-Transformer INT8        │
└─────────────────────────────────────┘
```

### 3.1 仓库结构

```
Vosi/
├── README.md / README.zh-CN.md
├── LICENSE                           # Apache-2.0
├── docs/
│   ├── specs/                        # 设计文档
│   └── guides/                       # 用户指南
├── src-tauri/                        # Rust 核心
│   ├── src/
│   │   ├── main.rs
│   │   ├── audio/                    # 采集 + VAD
│   │   ├── asr/                      # sherpa-onnx 封装
│   │   ├── hotkey/                   # 全局热键
│   │   ├── inject/                   # 文本注入 (platform-specific)
│   │   ├── post/                     # ITN + 热词后处理
│   │   └── config/                   # 用户配置
│   └── tauri.conf.json
├── ui/                               # 设置界面 (HTML/CSS/TS)
├── models/
│   └── manifest.json                 # 模型清单 + SHA256
└── scripts/
    └── download-models.sh
```

---

## 4. 核心组件规格

### 4.1 音频采集 (`audio/capture.rs`)

| 属性 | 规格 |
|------|------|
| 库 | `cpal` |
| 格式 | 16kHz / mono / f32 PCM |
| 缓冲 | 环形缓冲，chunk 100ms |
| 降噪 | v0.1 依赖 VAD 过滤静音段；v0.2 可加 RNNoise |

按住热键时启动采集，松手后停止并 flush 剩余缓冲。

### 4.2 VAD (`audio/vad.rs`)

| 属性 | 规格 |
|------|------|
| 模型 | sherpa-onnx Silero VAD 或 FSMN-VAD INT8 |
| 职责 | 过滤呼吸音/静音，动态截断句尾 |
| 可配置 | `silence_threshold_ms`（默认 800ms）、`min_speech_ms`（默认 300ms） |

### 4.3 ASR 推理 (`asr/engine.rs`)

| 属性 | 规格 |
|------|------|
| 引擎 | sherpa-onnx `OfflineRecognizer` (Paraformer) |
| 模型 | `sherpa-onnx-paraformer-zh` 或 FunASR 导出 `paraformer-large` INT8 |
| 线程 | 默认 2，可配置 1–4 |
| 模式 | 离线批识别（松手触发），v0.1 不做流式 |

**推理管线：**

```
音频 → Paraformer ASR → 原始文本 → Punc-CT-Transformer → 带标点文本 → ITN 规则引擎 → 最终文本
```

**ITN（v0.1）**：规则引擎处理数字、日期、金额等常见模式；v0.2 可换 FunASR ITN FST。

### 4.4 热词 (`post/hotword.rs`)

sherpa-onnx contextual biasing 仅支持 transducer 模型，Paraformer 不原生支持。

| 版本 | 方式 |
|------|------|
| v0.1 | 后处理文本模糊匹配替换 |
| v0.2 | 换 transducer 模型或 FunASR nn-hotword 推理 biasing |

热词文件：每行一个词/短语，路径 `~/.config/vosi/hotwords.txt`，支持导入/导出。

### 4.5 全局热键 (`hotkey/listener.rs`)

| 平台 | 实现 |
|------|------|
| macOS | `rdev` + 可选原生插件处理 Fn/CapsLock 冲突 |
| Windows | `rdev` 或 Win32 `RegisterHotKey` |

| 行为 | 说明 |
|------|------|
| 默认热键 | 右 Alt（可自定义） |
| 按住 | 开始录音，托盘图标变红 |
| 松开 | 触发 ASR → 后处理 → 上屏 |
| 短按（<300ms） | 忽略，防误触 |

### 4.6 文本注入 (`inject/`)

| 平台 | API | 权限 |
|------|-----|------|
| macOS | `CGEvent` 模拟键盘（Unicode） | 辅助功能（Accessibility） |
| Windows | `SendInput` | 无特殊权限 |

注入失败时 fallback：文本写入剪贴板 + Toast 提示。用户可选「直接键入」或「粘贴」模式。

### 4.7 配置管理 (`config/`)

配置文件路径：`~/.config/vosi/settings.toml`（macOS/Linux）、`%APPDATA%\Vosi\settings.toml`（Windows）

```toml
[hotkey]
trigger_key = "RightAlt"
mode = "hold"  # hold | toggle

[audio]
sample_rate = 16000
silence_threshold_ms = 800
min_speech_ms = 300

[asr]
num_threads = 2
mode = "short"  # short | long
model_variant = "paraformer-large-int8"

[hotword]
enabled = true
file = "~/.config/vosi/hotwords.txt"

[inject]
method = "type"  # type | paste

[general]
start_on_boot = true
show_tray = true
```

### 4.8 模型管理器 (`asr/model_manager.rs`)

- 检测模型目录：`~/.vosi/models/`（macOS/Linux）或 `%LOCALAPPDATA%\Vosi\models\`（Windows）
- 安装包内嵌 INT8 模型，首次启动解压到上述目录
- SHA256 完整性校验
- 开发者通过 `scripts/download-models.sh` 拉取模型

---

## 5. 数据流与延迟

### 5.1 按住说话流程

1. 用户按住热键 → 开始录音
2. 每 100ms 采集 PCM chunk → VAD 标记有效/静音
3. 用户松开热键 → 停止录音、flush 缓冲
4. 完整语音段送入 Paraformer 推理
5. 原始文本 → 标点模型 → 热词替换 → ITN
6. 最终文本注入焦点输入框

### 5.2 延迟预算

| 阶段 | 短句（<5s） | 长句（5–30s） |
|------|------------|--------------|
| 音频 flush | ~10ms | ~10ms |
| Paraformer 推理 | ~200ms | ~800ms |
| 标点推理 | ~50ms | ~150ms |
| 热词 + ITN | ~5ms | ~10ms |
| 文本注入 | ~20ms | ~50ms |
| **总计** | **~285ms** | **~1020ms** |

目标：短句松手到上屏 < 500ms；长句 < 1.5s。

### 5.3 长短文本模式

| | 短句模式 | 长文模式 |
|--|---------|---------|
| 触发 | 松手一次性识别 | 松手识别 + VAD 自动分段 |
| 上屏 | 整段一次性注入 | 按段注入，段间自动换行 |
| 适用 | 聊天、快捷回复 | 撰稿、会议记录 |

---

## 6. 错误处理与权限

### 6.1 权限

| 平台 | 权限 | 首次启动 |
|------|------|---------|
| macOS | 麦克风 | 系统弹窗自动请求 |
| macOS | 辅助功能 | 设置页引导跳转系统设置 |
| Windows | 麦克风 | 系统弹窗自动请求 |

权限缺失时：托盘警告图标，识别功能禁用，设置页展示修复指引。

### 6.2 错误处理

| 错误 | 处理 |
|------|------|
| 麦克风不可用 | Toast「未检测到麦克风」，托盘变黄 |
| 模型文件损坏 | 提示重新安装 |
| ASR 推理超时（>5s） | 取消推理，Toast「识别超时，请重试」 |
| 无语音内容 | 静默忽略，不上屏 |
| 文本注入失败 | fallback 剪贴板 + Toast |
| 热键冲突 | 设置页提示更换热键 |

### 6.3 日志（v0.1）

- 路径：`~/.vosi/logs/vosi.log`，滚动 5MB × 3 文件
- 不记录识别文本内容
- 仅记录：启动/停止、错误码、推理耗时

### 6.4 离线合规

- 禁止 HTTP 网络依赖（构建时静态分析）
- 不创建外部网络 socket
- 隐私声明：「Vosi 不收集、不上传任何数据」

---

## 7. 测试与打包

### 7.1 测试分层

| 层级 | 范围 | 工具 |
|------|------|------|
| 单元测试 | 热词替换、ITN 规则、配置解析 | Rust `#[test]` |
| 集成测试 | ASR 管线（固定 WAV → 期望文本） | integration tests + 黄金样本 |
| 平台测试 | 热键、注入、权限 | 手动测试清单 |
| 性能测试 | RTF、内存占用 | benchmark 脚本 |

黄金样本：10 条标准 WAV（短句/长句/数字/中英混输），CI 验证准确率 > 90%。

### 7.2 打包

| 平台 | 格式 | 目标体积 |
|------|------|---------|
| Windows | NSIS 单文件 EXE | < 400MB（含模型） |
| macOS | DMG（arm64 + x64 universal） | < 400MB（含模型） |

### 7.3 CI/CD

```
push/PR → cargo test → cargo clippy
tag release → build-windows + build-macos → GitHub Release 附件
```

### 7.4 开源发布清单

| 资产 | 说明 |
|------|------|
| README.zh-CN.md / README.md | 项目介绍、快速开始 |
| LICENSE | Apache-2.0 |
| docs/guides/quick-start.md | 安装与使用 |
| docs/guides/model-list.md | 模型来源与版本 |
| models/manifest.json | 模型清单 + SHA256 |
| .github/workflows/ | CI 构建 |

---

## 8. 版本路线图

| 版本 | 能力 |
|------|------|
| **v0.1** | 离线语音输入 + 热键 + 标点 + ITN + 热词替换 + 双平台打包 |
| v0.2 | 语音快捷指令、场景润色、方言模型、推理阶段热词 biasing |
| v0.3 | 会议说话人区分、创作润色、零日志模式 |

---

## 9. 决策记录

| 决策项 | 选择 | 理由 |
|--------|------|------|
| v0.1 范围 | 方案 B | 完整输入体验，不含指令/润色/方言 |
| 桌面壳 | Tauri 2 | 轻量、低内存、适合常驻后台 |
| ASR 运行时 | sherpa-onnx | 零 Python、双平台预编译 |
| 模型来源 | FunASR ONNX 生态 | 中文适配最佳 |
| 热词 v0.1 | 后处理替换 | Paraformer 不支持 sherpa hotword biasing |
| ITN v0.1 | 规则引擎 | 避免 FST 依赖，够用 |
| 分发 | EXE + DMG 内嵌模型 | 开箱即用 |
