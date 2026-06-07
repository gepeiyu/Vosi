# Vosi 快速上手

## 环境要求

- macOS 12+ 或 Windows 10+
- [Node.js](https://nodejs.org/) 20+（开发构建）
- [Rust](https://rustup.rs/) stable（开发构建）

## 开发构建

```bash
git clone https://github.com/gepeiyu/Vosi.git
cd Vosi
npm install

# 下载 ASR 模型（约 360 MB，仅开发时需要）
# 有本地代理时推荐：
export VOSI_PROXY=http://127.0.0.1:7890
./scripts/download-models.sh

# 首次编译需下载 sherpa-onnx 静态库（~18 MB）
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"

cd src-tauri && cargo test --lib && cd ..
npm run tauri dev
# debug 构建会自动从 models/dev/ 安装模型到 Application Support
```

## Release 安装包构建

在**对应平台**上从源码打包（Mac 无法直接出 Windows 包）：

```bash
./scripts/download-models.sh
./scripts/prepare-bundle-models.sh    # 复制模型到 src-tauri/models/bundled/
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
npm run tauri build
```

| 平台 | 产物路径 |
|------|----------|
| macOS | `src-tauri/target/release/bundle/dmg/*.dmg` |
| Windows | `src-tauri/target/release/bundle/nsis/*.exe` |

GitHub 上打 `v*` tag 可自动构建 **macOS arm64 / macOS Intel / Windows x64** 三种安装包并发布到 Releases（见 `docs/logs/2026-06-07-v01-polish-wrap-up.md`）。

## 首次使用（终端用户）

1. 安装 `.dmg`（macOS）或 `.exe`（Windows）安装包。
2. 启动后查看系统托盘中的 Vosi 图标。
3. **macOS**：在「系统设置 → 隐私与安全性」中授权 **麦克风** 和 **辅助功能**。
4. **Windows**：在「设置 → 隐私 → 麦克风」中允许桌面应用访问麦克风。
5. 打开任意文本编辑器，**按住触发键**（macOS 默认右 Command ⌘，Windows 默认右 Alt）**至少 0.3 秒**，说话，松手后文字自动输入。
6. **轻点**触发键（< 300 ms）不会启动录音。

> macOS 仅监听**配置侧** Command 键（默认右 ⌘）；左 ⌘ 用于 ⌘+C 等快捷键，不会唤起 Vosi。

## 设置

右键托盘 → **设置**，可修改：

| 项 | 说明 |
|----|------|
| 触发键 | macOS 右 Command；Windows 右 Alt |
| ASR 模式 | 短句 / 长句 |
| 静音阈值 | VAD 截断灵敏度（毫秒） |
| 注入方式 | 模拟键盘 / 剪贴板粘贴 |
| 热词文件 | 每行一个热词，用于后处理替换 |

配置保存在 `~/.config/vosi/settings.toml`（macOS/Linux）或 `%APPDATA%\vosi\`（Windows）。

## 日志

运行日志位于：

- macOS：`~/Library/Application Support/vosi/logs/vosi.log`
- Windows：`%APPDATA%\vosi\logs\vosi.log`

日志**不包含**识别文本内容，仅记录操作元数据（推理耗时、采样数等）。

## 故障排查

| 现象 | 处理 |
|------|------|
| 按住热键无反应 | 检查是否有其他软件占用全局热键；macOS 需辅助功能权限 |
| 左 ⌘ 误触发 | 确认设置中触发键为「右 Command」；左 ⌘ 不应唤起 Vosi |
| 轻点也出现浮窗 | 应已修复：需按住 ≥ 300 ms；若仍出现请重编后重启 |
| 有录音无文字 | 确认模型已安装；查看日志 `inference_ms` |
| 文字未注入 | macOS 需辅助功能权限；Windows 需以普通用户运行 |
| 编译失败 sherpa-onnx | 设置 `SHERPA_ONNX_ARCHIVE_DIR` 或使用稳定网络 |

更多手动测试项见 [manual-test-checklist.md](./manual-test-checklist.md)。  
v0.1.1 收尾说明见 [2026-06-07-v01-polish-wrap-up.md](../logs/2026-06-07-v01-polish-wrap-up.md)。  
完整项目总结见 [PROJECT-SUMMARY.md](../PROJECT-SUMMARY.md)。
