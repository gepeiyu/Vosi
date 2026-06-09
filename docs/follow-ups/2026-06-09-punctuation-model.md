# 待完成：接入离线标点模型

## 背景

当前识别链路使用 SenseVoice 输出文本，后处理只包含热词替换和简单 ITN：

- `src-tauri/src/asr/engine.rs`：调用 SenseVoice 获取原始识别文本
- `src-tauri/src/post/pipeline.rs`：执行 trim、热词替换、ITN
- `src-tauri/src/post/itn.rs`：数字和日期等简单转换

因此输入文本经常没有逗号、句号、问号等自然标点。之前的 legacy `punctuation` 模型目录已在模型迁移逻辑中清理，不再随当前包分发。

## 目标

接入本地离线标点恢复能力，在不依赖云服务的前提下，为 ASR 输出补全常见标点。

## 待确认

- 使用 sherpa-onnx 提供的 punctuation API，还是单独引入其他 ONNX 标点模型。
- 标点模型来源、许可证、体积和多语言支持范围。
- 是否只支持中文，还是覆盖中文/英文/日文。
- 标点处理是否默认开启，是否需要在设置页提供开关。

## 实施清单

- [ ] 在 `models/manifest.json` 增加 punctuation 模型条目。
- [ ] 更新 `scripts/download-models.sh`，支持下载标点模型。
- [ ] 更新 `scripts/prepare-bundle-models.sh`，将标点模型复制到 `src-tauri/models/bundled/`。
- [ ] 在 `src-tauri/src/asr/model_manager.rs` 增加标点模型就绪检查和安装复制逻辑。
- [ ] 新增 `src-tauri/src/post/punctuation.rs`，封装标点模型加载与推理。
- [ ] 在 `src-tauri/src/post/pipeline.rs` 中接入标点恢复，顺序建议为：热词替换 → ITN → 标点恢复。
- [ ] 增加配置项，例如 `post.punctuation_enabled = true`。
- [ ] 在设置页增加标点开关文案和持久化逻辑。
- [ ] 更新手动测试清单，覆盖短句、长句、中英混说和无标点输入。
- [ ] 验证 macOS / Windows 打包体积和首次启动模型安装流程。

## 验收标准

- [ ] 断网状态下可完成标点恢复。
- [ ] 常见中文陈述句自动补句号或逗号。
- [ ] 疑问语气能补问号，至少不明显误伤普通陈述句。
- [ ] 长句模式分段后仍能输出可读文本。
- [ ] 标点失败时不影响 ASR 原文上屏。
