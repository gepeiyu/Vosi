# Vosi

**Voice Operational Speech Input** — 全离线桌面语音输入工具（Windows / macOS）。

按住热键说话、松手即上屏。100% 离线，用户侧零 Python 依赖。

Powered by FunASR 模型，基于 sherpa-onnx 推理。

![License](https://img.shields.io/badge/license-Apache--2.0-blue)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-lightgrey)
![Release](https://img.shields.io/github/v/release/gepeiyu/Vosi?label=v0.1.0)

## 功能（v0.1）

- 全局热键按住说话
- Paraformer 识别 + 标点 + ITN 数字归一
- 热词后处理替换
- 浮动录音胶囊（音量条 + 识别中状态）
- 长句模式：Silero VAD 分段，多句以换行拼接
- 错误系统通知（麦克风不可用、识别超时、注入失败回退剪贴板）
- 托盘三态图标（就绪 / 录音 / 警告）
- 完整设置界面（语音阈值、线程数、热词、胶囊、托盘等）
- 隐私安全日志（不记录识别文本）

## 快速开始

```bash
npm install
./scripts/download-models.sh
export SHERPA_ONNX_ARCHIVE_DIR="$PWD/.cache/sherpa-onnx"
npm run tauri dev
```

详细说明：[docs/guides/quick-start.md](docs/guides/quick-start.md)

## 文档

- [**项目总结**](docs/PROJECT-SUMMARY.md) — 开发、模型、权限、发版一站式参考
- [快速上手](docs/guides/quick-start.md)
- [设计规格](docs/specs/2026-06-05-vosi-v01-design.md)
- [模型清单](docs/guides/model-list.md)
- [手动测试清单](docs/guides/manual-test-checklist.md)
- [文档索引](docs/README.md)

## 许可证

Apache-2.0 — 见 [LICENSE](LICENSE)。
