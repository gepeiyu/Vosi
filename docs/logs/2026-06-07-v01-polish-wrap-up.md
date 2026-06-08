# Vosi v0.1.1 收尾总结

> 日期：2026-06-08（更新） | 分支：`feat/v0.1.1-polish`

本文档汇总 v0.1.1 polish 阶段的功能修复、品牌资源、打包发布与开发运维信息，供后续维护与发版参考。

---

## 1. 当前状态

| 项 | 状态 |
|----|------|
| 功能开发 | v0.1.1 polish + SenseVoice 升级已完成 |
| CI | `main` / PR 上 macOS clippy + unit tests 已通过 |
| PR | [#1](https://github.com/gepeiyu/Vosi/pull/1)（`feat/v0.1.1-polish` → `main`） |
| **应用版本号** | **`0.1.0`**（`tauri.conf.json` / `Cargo.toml` / `package.json`） |
| **本地 DMG** | `src-tauri/target/release/bundle/dmg/Vosi_0.1.0_x64.dmg`（2026-06-08，Intel macOS） |
| 版本对应原则 | 安装包文件名 `Vosi_<version>_*.dmg` 须与 `tauri.conf.json` 的 `version` 一致；改版本后须全量 `npm run tauri build`（`--bundles app` 不更新 DMG） |
| ASR 模型 | SenseVoice INT8（`sense-voice/`）；已移除 legacy `paraformer-zh/` |
| 打包体积 | 含标点约 **~510MB**（sense-voice 228MB + punctuation 281MB + VAD） |
| GitHub Release | 打 `v*` tag 后由 `.github/workflows/release.yml` 自动构建并发布 |

---

## 2. 行为与交互变更

### 2.1 按住说话（300 ms 阈值）

- **轻点（< 300 ms）**：无任何反应（不显示浮窗、不开麦、不识别）。
- **按住 ≥ 300 ms**：才开始录音并显示浮窗；松手后识别并注入。
- 实现：`src-tauri/src/lib.rs` 中 `MIN_HOLD_MS = 300`，通过 `PipelineEvent::BeginRecording` 延迟启动，而非「按下即录、短按再取消」。

### 2.2 macOS 热键（仅监听配置侧 Command）

- 默认触发键：**右 Command**（keycode `54`）。
- **左 Command**（keycode `55`）不再触发，避免 ⌘+C 等快捷键误唤起。
- 实现：`src-tauri/src/hotkey/macos.rs` — `RightCommand` → `[54]`，`LeftCommand` → `[55]`。

### 2.3 浮窗与焦点（已知限制）

- 录音时 `overlay.show()` 在 macOS 上仍可能激活 Vosi 应用，导致原输入框失焦。
- `tauri.conf.json` 中 overlay 已设 `"focus": false`，但 macOS 对非激活窗口支持有限。
- **后续优化方向**：启动时完成权限引导；overlay 改为 `NSPanel` + `NonActivatingPanel`；保持 `ActivationPolicy::Accessory`。

### 2.4 其他已修复项（本阶段早期）

- CGEventTap 替代 rdev，修复 macOS 热键崩溃。
- 浮窗位置/尺寸、电平推送、点按误触取消等（见 `docs/logs/2026-06-06-vosi-v01-polish-execution-log.md`）。

---

## 3. 品牌与图标

### 3.1 设计

- **概念**：透明底 + 白色胶囊 + 红点（录音）+ 蓝色音量柱（V 形，色值 `#0A84FF`）。
- **App 图标**（Dock / 安装包）：胶囊占画布 **82% × 58%**。
- **状态栏托盘**：单独大图，胶囊占 **94% × 78%**，与菜单栏邻图标高度更协调；红点/音量柱在胶囊内居中放大。

### 3.2 文件与脚本

| 用途 | 路径 |
|------|------|
| App 源图（1024） | `assets/vosi-logo-1024-transparent.png` |
| 托盘源图（1024） | `assets/vosi-tray-1024-transparent.png` |
| 生成脚本 | `scripts/generate-logo.py` |
| App / bundle | `src-tauri/icons/icon.png`、`.icns`、`.ico` |
| 托盘三态 | `icon-idle.png`、`icon-recording.png`、`icon-warning.png` |

### 3.3 重新生成图标

```bash
# 1. 生成 App + 托盘 PNG（需 Python 3 + Pillow）
python3 scripts/generate-logo.py

# 2. 生成 .icns / .ico / 多尺寸（会覆盖 icon.png，脚本第 2 步会再写回 tray 图）
npx tauri icon assets/vosi-logo-1024-transparent.png -o src-tauri/icons
python3 scripts/generate-logo.py

# 3. 修改图标后需重编 Rust（include_image! 嵌入二进制）
# build.rs 已对 icon-*.png 声明 cargo:rerun-if-changed
npm run tauri dev   # 或 cargo build
```

### 3.4 配置

- `tauri.conf.json` → `trayIcon.iconPath`: `icons/icon-idle.png`
- `trayIcon.iconAsTemplate`: `false`（彩色托盘图标）
- 运行时三态切换：`src-tauri/src/app/tray.rs` → `include_image!`

---

## 4. 模型打包

### 4.1 问题与修复

- 原配置 `"models/bundled/*"` 只复制顶层文件，**模型目录未打入安装包**（仅 `.gitkeep`）。
- 已改为 `"models/bundled/"` 递归打包（Tauri 2 目录递归语法）。

### 4.2 Release / 本地构建前

```bash
./scripts/download-models.sh          # → models/dev/
./scripts/prepare-bundle-models.sh    # → src-tauri/models/bundled/
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
npm run tauri build
```

- **`models/dev/`** 与 **`src-tauri/models/bundled/*`** 不入 Git（约 510 MB）；构建前本地或 CI 执行上述脚本。
- 用户数据目录安装副本：`~/Library/Application Support/vosi/models/`（macOS）。

---

## 5. 打包与发布

### 5.1 本地可打安装包

| 平台 | 在哪构建 | 产物 |
|------|----------|------|
| macOS Intel `.dmg` | Intel Mac | `src-tauri/target/release/bundle/dmg/*.dmg` |
| macOS Apple Silicon `.dmg` | M 系列 Mac | 同上 |
| Windows `.exe` | Windows | `src-tauri/target/release/bundle/nsis/*.exe` |

原则：**在目标系统上** `npm run tauri build`；Mac 上无法直接出 Windows 包。

### 5.2 GitHub Actions Release

触发：推送 tag `v*`（如 `v0.1.1`）。

| Job | Runner | 产物 |
|-----|--------|------|
| `build-macos` (arm64) | `macos-latest` | `vosi-macos-arm64` → `.dmg` |
| `build-macos` (x64) | `macos-15-intel` | `vosi-macos-x64` → `.dmg` |
| `build-windows` | `windows-latest` | `vosi-windows-x64` → `.exe` |
| `publish` | `ubuntu-latest` | 上传至 GitHub Release |

Workflow：`.github/workflows/release.yml`

### 5.3 发版检查清单

- [ ] 合并 PR 到 `main`（或确认 tag 指向含 workflow 的 commit）
- [ ] bump 版本号（若发 v0.1.1）
- [ ] `git tag v0.1.1 && git push origin v0.1.1`
- [ ] 在 GitHub Releases 下载三平台安装包并实机验证
- [ ] 参考 [manual-test-checklist.md](../guides/manual-test-checklist.md)

---

## 6. 开发与运行

```bash
npm install
./scripts/download-models.sh
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
npm run tauri dev
```

- Debug 模式从 `models/dev/` 安装模型到 Application Support。
- 日志：`~/Library/Application Support/vosi/logs/vosi.log`（不含识别文本）。
- 配置：`~/.config/vosi/settings.toml`

### 权限（macOS）

| 权限 | 用途 | 申请时机 |
|------|------|----------|
| 麦克风 | 录音 | 启动时 `AudioCapture::preflight_microphone()` 触发系统弹窗 |
| 辅助功能 | CGEventTap 热键 + enigo 文本注入 | 启动时 `AXIsProcessTrustedWithOptions(prompt=true)` 弹出系统引导；未授权时热键线程每 2s 重试 |

实现：`src-tauri/src/permissions/macos.rs`（`ensure_at_launch`）。

- `src-tauri/Info.plist` 含 `NSMicrophoneUsageDescription`（Release 必需）。
- 未授权时托盘切为警告态并发送系统通知；设置页提供「麦克风设置」「辅助功能设置」按钮。

---

## 7. SenseVoice 升级（2026-06-07–08）

- 规格：`docs/specs/2026-06-07-vosi-model-upgrade-design.md`
- 执行日志：`docs/logs/2026-06-07-sensevoice-model-upgrade-execution-log.md`
- 默认 ASR：`language=auto`、`use_itn=true`、`punctuation_enabled=true`
- Task 10 已清理 paraformer legacy；标点管线保留
- Golden 15 条 WAV / 标点对比：**暂缓**（不阻塞当前版本）

---

## 8. 后续待办

| 项 | 文档 |
|----|------|
| 开机自启平台注册 | `docs/follow-ups/2026-06-06-start-on-boot-platform-registration.md` |
| 非激活浮窗（NSPanel） | 本文 §2.3 |
| bump 版本号 + 打 tag 发 GitHub Release | 本文 §5（当前仍为 0.1.0） |
| merge PR #1 | GitHub |
| Golden WAV + 标点对比（可选） | `tests/fixtures/audio/README.md` |

---

## 9. 关键文件索引

```
src-tauri/src/lib.rs              # 300ms hold-to-talk 管线
src-tauri/src/hotkey/macos.rs     # 左右 Command 分离
src-tauri/src/overlay/mod.rs      # 浮窗显示与定位
src-tauri/src/app/tray.rs         # 托盘三态图标
src-tauri/tauri.conf.json         # bundle resources、tray、overlay
src-tauri/build.rs                # icon 变更触发重编
scripts/generate-logo.py          # Logo 生成
scripts/prepare-bundle-models.sh  # Release 模型复制
.github/workflows/release.yml     # 三平台 Release
.github/workflows/ci.yml          # PR / main CI
```

---

## 10. 相关文档

- 设计规格：`docs/specs/2026-06-06-vosi-v01-polish-design.md`
- 实施计划：`docs/plans/2026-06-06-vosi-v01-polish.md`
- 执行日志：`docs/logs/2026-06-06-vosi-v01-polish-execution-log.md`
- 快速上手：`docs/guides/quick-start.md`
- 手动测试：`docs/guides/manual-test-checklist.md`
