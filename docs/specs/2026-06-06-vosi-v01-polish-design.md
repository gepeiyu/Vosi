# Vosi v0.1.1 收尾设计规格

> **Voice Operational Speech Input** — v0.1 功能缺口补齐
>
> 日期：2026-06-06 | 状态：已评审待实施
>
> 前置规格：`docs/specs/2026-06-05-vosi-v01-design.md`

## 1. 背景与目标

### 1.1 动机

v0.1 核心管线（16 个 Task）已完成，但对照原始设计规格与实机体验，存在以下缺口：

| 领域 | 设计规格 | 当前实现 |
|------|----------|----------|
| 长文模式 | VAD 分段 + 按段上屏换行 | UI 有选项，管线未读取 `asr.mode` |
| VAD | `audio/vad.rs` 静音截断 | 模块不存在 |
| 错误反馈 | Toast + 托盘状态图标 | 仅日志 + tooltip |
| 注入失败 | 剪贴板 fallback + Toast | 仅记日志 |
| 设置 UI | 线程数、最短语音、热词开关等 | 仅 5 个基础字段 |
| 按住动效 | — | 无（用户新增需求，参考 Typeless） |

### 1.2 v0.1.1 范围

**纳入：**

1. **核心管线**：Silero VAD 长文分段、注入剪贴板 fallback、ASR 超时取消（5s）
2. **用户反馈**：系统通知（错误路径）、托盘多态图标（就绪/录音/警告）
3. **设置 UI 补全**：线程数、最短语音、热词开关、托盘显示、开机自启（配置持久化）、胶囊开关
4. **浮动胶囊动效**：按住显示音量条，松手切换「识别中…」脉冲，完成后隐藏

**明确排除（仍属 v0.2+ 或 follow-up）：**

- 语音快捷指令、场景润色、方言模型
- 流式 ASR 实时预览
- 开机自启的平台注册（LaunchAgent / 注册表）— 本轮仅 UI + 配置字段
- Golden WAV 本地录制、Release 打包实机验证（不在本轮自动化范围，保留手动清单）

### 1.3 成功标准

- 短句模式：按住 → 胶囊 + 音量条 → 松手 →「识别中…」→ 文本上屏 → 胶囊隐藏，全程 < 500ms（短句）
- 长句模式：30s 语音经 VAD 切分为多段，上屏文本含换行分隔
- 注入失败：文本自动写入剪贴板 + 系统通知，用户可手动粘贴
- 设置页所有 `settings.toml` 字段可编辑并持久化
- macOS + Windows 双平台 overlay 窗口正常显示（透明、置顶、点击穿透）

---

## 2. 技术选型

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 浮动胶囊 | Tauri 2 透明置顶 WebView 窗口 | 与现有栈一致，一套 UI 覆盖双平台 |
| VAD | sherpa-onnx Silero VAD（已有 manifest） | 与设计规格一致，模型路径已在 `ModelPaths` |
| 长文分段 | 松手后 VAD 切分 → 逐段离线 ASR | Paraformer 为批识别，不适合录音中流式分段 |
| 音量电平 | 录音线程 RMS 计算，50ms 推送 | 轻量，无需额外依赖 |
| 系统通知 | `tauri-plugin-notification` | Tauri 官方插件，跨平台 |
| 托盘图标 | 三套 PNG + `TrayIcon::set_icon` | 直观状态反馈 |

---

## 3. 系统架构

```
热键线程 (rdev)
    │ HotkeyEvent::Pressed / Released
    ▼
VoicePipeline (Rust 后台线程)
    ├─ AudioCapture (+ RMS level channel)
    ├─ VadEngine（仅 long 模式）
    ├─ AsrEngine + PunctuationEngine
    ├─ PostProcess (ITN + Hotword)
    ├─ TextInjector (+ clipboard fallback)
    └─ OverlayController + Notifier
            │ app.emit("overlay-state", payload)
            ▼
    overlay 窗口 (WebView)
    main 窗口 (设置页，按需显示)
    托盘图标 (idle / recording / warning)
```

### 3.1 胶囊状态机

```
Hidden
  └─[热键按下]─► Recording（音量条动画，50ms 电平更新）
       └─[热键松开]─► Processing（「识别中…」脉冲）
            ├─[上屏成功]─► Hidden
            ├─[无语音/过短]─► Hidden（跳过 Processing 或瞬间隐藏）
            └─[失败/超时]─► Hidden + 通知 + 托盘 Warning
```

用户确认：**方案 B** — 松手后胶囊切换为「识别中…」，上屏或失败后隐藏。

### 3.2 新增/修改模块

| 路径 | 职责 |
|------|------|
| `src-tauri/src/audio/vad.rs` | Silero VAD 封装，语音段切分 |
| `src-tauri/src/audio/level.rs` | RMS 电平计算 |
| `src-tauri/src/audio/capture.rs` | 扩展：录音时推送电平样本 |
| `src-tauri/src/overlay/mod.rs` | 窗口创建、定位、状态 emit |
| `src-tauri/src/notify/mod.rs` | 系统通知封装 |
| `src-tauri/src/pipeline/session.rs` | 长文分段、超时、模式分支 |
| `src-tauri/src/inject/mod.rs` | `inject_with_fallback()` |
| `src/overlay.html` + `src/overlay.ts` + `src/overlay.css` | 胶囊 UI |
| `src/index.html` + `src/main.ts` | 设置 UI 补全 |
| `src-tauri/icons/icon-*.png` | 托盘三态图标 |

---

## 4. 浮动胶囊 UI 规格

### 4.1 窗口属性（`overlay` label）

| 属性 | 值 |
|------|-----|
| 尺寸 | 280 × 56 px |
| 位置 | 主显示器底部居中，距底边 48px |
| `transparent` | true |
| `decorations` | false |
| `alwaysOnTop` | true |
| `skipTaskbar` | true |
| `visible` | false（默认隐藏，录音时 show） |
| 交互 | CSS `pointer-events: none`（点击穿透） |

### 4.2 视觉

**Recording 状态：**

- 毛玻璃胶囊背景（`backdrop-filter: blur(20px)`），圆角 28px
- 左侧：红色圆点，opacity 0.6↔1.0 循环（1s 周期）
- 品牌字「Vosi」
- 右侧：5 条竖向音量柱，高度映射 `level`（0.0–1.0），最小高度 20%

**Processing 状态：**

- 左侧：灰色脉冲圆环
- 文案：「识别中…」
- 右侧：三点 stagger 呼吸动画
- 无音量柱

**过渡：** 状态切换 150ms ease；显示/隐藏 200ms opacity fade。

### 4.3 事件协议

```typescript
type OverlayPayload =
  | { phase: "hidden" }
  | { phase: "recording"; level: number }
  | { phase: "processing" };
```

- 事件名：`overlay-state`
- 方向：Rust → overlay WebView（`app.emit_to("overlay", ...)`）
- 电平频率：50ms（仅 Recording 阶段）

### 4.4 配置

```toml
[overlay]
enabled = true  # false 时跳过 overlay 窗口操作，管线行为不变
```

---

## 5. 核心管线变更

### 5.1 短句模式（`asr.mode = "short"`）

与现有流程一致，整段一次识别：

1. 按住 → 开始录音 + 胶囊 Recording
2. 松手 → 胶囊 Processing
3. `finalize_recording()` 整段 ASR → 标点 → 后处理
4. 注入 → 隐藏胶囊

### 5.2 长句模式（`asr.mode = "long"`）

松手后增加 VAD 分段步骤：

1. 获取完整 PCM 样本
2. `VadEngine::segment(samples, silence_threshold_ms)` → `Vec<Segment>`
3. 对每个 segment：ASR → 标点 → 后处理
4. 段间以 `\n` 拼接为最终文本
5. 一次性注入（避免段间焦点丢失）

**VAD 参数：**

- 模型：`ModelPaths.vad_model`（`vad/model.onnx`，Silero）
- `silence_threshold_ms`：来自 `config.audio.silence_threshold_ms`（默认 800）
- `min_speech_ms`：短于该值的 segment 丢弃

### 5.3 音频电平

```rust
// 每 50ms 从最近 100ms 样本计算 RMS
fn rms_level(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt().clamp(0.0, 1.0)
}
```

录音线程通过 `mpsc::Sender<f32>` 推送电平；主循环转发给 overlay。

### 5.4 ASR 超时

- 阈值：5 秒（硬编码，v0.1.1 不做配置项）
- 实现：`std::thread::scope` 或 `recv_timeout` 包装推理调用
- 超时行为：取消推理、通知「识别超时，请重试」、托盘 Warning 3s 后恢复 Idle

### 5.5 注入 fallback

```rust
fn inject_with_fallback(injector, text, method) -> Result<(), String> {
    match injector.inject(text, method) {
        Ok(()) => Ok(()),
        Err(e) => {
            arboard::Clipboard::new()?.set_text(text)?;
            notifier.error("已复制到剪贴板，请手动粘贴");
            Err(e)
        }
    }
}
```

---

## 6. 错误处理

| 场景 | 胶囊 | 托盘 | 系统通知 |
|------|------|------|----------|
| 麦克风不可用 | 不显示 | Warning（持续） | 「未检测到麦克风」 |
| 录音过短（< min_speech_ms） | 直接隐藏 | Idle | 无 |
| ASR 结果为空 | Processing → 隐藏 | Idle | 无 |
| ASR 超时（>5s） | Processing → 隐藏 | Warning 3s → Idle | 「识别超时，请重试」 |
| 文本注入失败 | Processing → 隐藏 | Warning 3s → Idle | 「已复制到剪贴板，请手动粘贴」 |
| 模型加载失败 | 不显示 | Warning（持续） | 「语音引擎不可用，请重新安装」 |
| 辅助功能未授权（macOS） | 不显示 | Warning（持续） | 无（设置页 banner） |

**原则：** 成功上屏不弹系统通知；仅错误路径触发通知。

### 6.1 托盘图标

| 状态 | 图标文件 | 触发 |
|------|----------|------|
| Idle | `icon-idle.png` | 就绪 |
| Recording | `icon-recording.png` | 按住热键 |
| Warning | `icon-warning.png` | 权限/错误 |

`TrayStatus` 枚举已有，补齐 `set_icon()` 与 `set_tooltip()` 联动。

---

## 7. 设置 UI 补全

### 7.1 新增字段

| 配置路径 | 控件 | 默认值 | 分组 |
|----------|------|--------|------|
| `audio.min_speech_ms` | number 100–1000 | 300 | 语音 |
| `asr.num_threads` | select 1/2/3/4 | 2 | 识别 |
| `hotword.enabled` | checkbox | true | 识别 |
| `overlay.enabled` | checkbox | true | 通用 |
| `general.show_tray` | checkbox | true | 通用 |
| `general.start_on_boot` | checkbox | true | 通用 |

### 7.2 配置结构扩展

```toml
[overlay]
enabled = true

# 其余字段保持不变，见 v0.1 规格 4.7 节
```

`AppConfig` 新增 `OverlayConfig { enabled: bool }`，`Default` 中 `enabled = true`。

旧配置无 `[overlay]` 段时，`load()` 迁移逻辑补默认值。

### 7.3 开机自启

本轮：**仅 UI + `settings.toml` 持久化**。

平台注册（macOS LaunchAgent、`HKCU\...\Run`）标注为 follow-up task，不阻塞 v0.1.1 发布。

---

## 8. 依赖变更

```toml
# Cargo.toml 新增
tauri-plugin-notification = "2"

# package.json 无需新增前端依赖
```

`tauri.conf.json` 新增 overlay 窗口定义；`capabilities/default.json` 为 overlay 窗口授权 notification 与 event 监听。

---

## 9. 测试策略

| 层级 | 范围 | 方式 |
|------|------|------|
| 单元测试 | RMS 电平、VAD 分段边界、fallback 逻辑 | Rust `#[test]` |
| 集成测试 | 长文模式分段 + 多段文本拼接 | `asr_pipeline` 扩展 |
| 手动测试 | 胶囊动效、通知、托盘图标、设置持久化 | 更新 `manual-test-checklist.md` |

不新增 E2E 浏览器测试（overlay 为原生窗口，手动验证为主）。

---

## 10. 决策记录

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 胶囊实现 | Tauri WebView 透明窗口 | 跨平台一套代码，迭代快 |
| VAD 方案 | sherpa Silero | 模型已在 manifest，规格一致 |
| 松手后动效 | 切换「识别中…」（方案 B） | 用户确认，接近 Typeless 体验 |
| 长文上屏 | 段间 `\n` 一次性注入 | 避免段间焦点丢失 |
| 通知策略 | 仅错误路径 | 减少打扰 |
| 开机自启 | 配置先行，平台注册 follow-up | 控制范围，不阻塞主功能 |

---

## 11. 与 v0.1 规格的关系

本规格为 `2026-06-05-vosi-v01-design.md` 的**增量补丁**，不修改 v0.1 已交付模块的既有行为（除明确列出的管线分支外）。v0.2 路线图（语音指令、润色、方言）不受影响。
