# Vosi 模型清单

Vosi v0.1 使用 FunASR 生态导出的 ONNX 模型，通过 [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) 在本地推理。

## 运行时版本

| 组件 | 版本 |
|------|------|
| sherpa-onnx (Rust crate) | 1.13.2 |
| 推理后端 | ONNX Runtime (CPU, 静态链接) |

## 模型列表

| 角色 | 模型 ID | 来源 | 说明 |
|------|---------|------|------|
| ASR | paraformer-zh-int8 | [sherpa-onnx ASR models](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models) | FunASR Paraformer 中文 INT8 |
| VAD | silero-vad | 同上 | 静音检测（v0.1 管线预留） |
| 标点 | punc-ct-transformer | [sherpa-onnx punctuation models](https://github.com/k2-fsa/sherpa-onnx/releases/tag/punctuation-models) | CT-Transformer 中英标点 |

完整 URL 与 SHA256 见 [`models/manifest.json`](../../models/manifest.json)。

## 下载

**重要**：ASR 与标点须使用 **sherpa-onnx 预打包** 的 ONNX（含 `vocab_size` / `tokens` metadata）。魔搭上的 FunASR 原生 ONNX（`model_quant.onnx`）与 sherpa-onnx **不兼容**。

```bash
# 推荐：本地 VPN 代理 + HuggingFace / GitHub（最快、格式正确）
export VOSI_PROXY=http://127.0.0.1:7890
./scripts/download-models.sh

# 无代理：HuggingFace 国内镜像
VOSI_MODEL_MIRROR=hf-mirror ./scripts/download-models.sh

# 强制走 GitHub Releases（需代理）
export VOSI_PROXY=http://127.0.0.1:7890
VOSI_MODEL_MIRROR=github ./scripts/download-models.sh
```

| 组件 | 来源 | 说明 |
|------|------|------|
| ASR | [csukuangfj/sherpa-onnx-paraformer-zh-small](https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-small-2024-03-09) | sherpa INT8，~78MB |
| 标点 | [csukuangfj/sherpa-onnx-punct-ct-transformer](https://huggingface.co/csukuangfj/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12) | sherpa 格式，~280MB |
| VAD | GitHub `silero_vad.onnx` | ~2MB；魔搭 FSMN 为备选 |

模型安装到 `models/dev/`，发布构建前执行：

```bash
./scripts/prepare-bundle-models.sh
```

将模型复制到 `src-tauri/models/bundled/` 并打入安装包。

## FunASR 溯源

- **Paraformer**：阿里达摩院 FunASR 中文语音识别模型
- **标点模型**：FunASR CT-Transformer 标点恢复
- **导出格式**：ONNX（由 sherpa-onnx 项目预导出，非 Python FunASR SDK 运行时）

Vosi 不在用户侧运行 Python 或 FunASR 代码，仅加载 ONNX 权重。

## 热词说明

Paraformer 在 sherpa-onnx 中不支持 runtime hotword biasing。Vosi v0.1 采用**后处理文本替换**（见 `src-tauri/src/post/hotword.rs`），在 ITN 之前对识别结果做模糊匹配替换。

## 许可证

模型权重遵循各自上游许可（通常为 Apache-2.0）。Vosi 应用本身为 Apache-2.0，见 [LICENSE](../../LICENSE)。
