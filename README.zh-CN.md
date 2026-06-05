# Vosi

**Voice Operational Speech Input** — 全离线桌面语音输入工具（Windows / macOS）。

按住热键说话、松手即上屏。100% 离线，用户侧零 Python 依赖。

Powered by FunASR 模型，基于 sherpa-onnx 推理。

![License](https://img.shields.io/badge/license-Apache--2.0-blue)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-lightgrey)

## 功能（v0.1）

- 全局热键按住说话
- Paraformer 识别 + 标点 + ITN 数字归一
- 热词后处理替换
- 托盘 + 设置界面
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

- [设计规格](docs/specs/2026-06-05-vosi-v01-design.md)
- [实现计划](docs/plans/2026-06-05-vosi-v01.md)
- [模型清单](docs/guides/model-list.md)
- [手动测试清单](docs/guides/manual-test-checklist.md)

## 许可证

Apache-2.0 — 见 [LICENSE](LICENSE)。
