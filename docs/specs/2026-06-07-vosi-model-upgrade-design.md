# Vosi 模型升级设计规格

> **ASR 模型替换：SenseVoice 三语混说**
>
> 日期：2026-06-07 | 状态：已评审待实施

## 1. 背景与动机

### 1.1 问题

当前 ASR 模型 `sherpa-onnx-paraformer-zh-small`（INT8，~78MB）在以下场景识别不足：

- 中文句中夹杂英文技术词时，常输出谐音汉字（如「瑞爱克特」而非 `React`）
- 无法满足程序员（框架名、API、缩写）和产品经理（MVP、PRD、KPI）的日常输入
- 不支持日语；用户需要中英日三语混说

### 1.2 目标

将 ASR 升级为面向程序员/产品经理的多语言模型，满足：

| 维度 | 要求 |
|------|------|
| 主语言 | 中文为主 |
| 英文 | 技术词输出正确拼写 |
| 日语 | 支持句中夹杂日语词/短语 |
| 混说模式 | 中英日三语可在同一句混说 |
| 离线 | 100% 本地推理，零网络 |
| 体积 | 以实测达标为准，尽量控制（目标 ≤ 300MB，上限 ≤ 550MB） |

### 1.3 迁移原则

- 新模型通过 golden + 人工验收后，再清理旧模型
- 测试期间保留 `paraformer-zh-small` 备份，不立即删除
- 验收通过后完全替换，不保留双模型切换 UI

---

## 2. 模型选型

### 2.1 主选：SenseVoice INT8

| 属性 | 值 |
|------|-----|
| 模型 ID | `sense-voice-zh-en-ja-ko-yue-int8` |
| 包名 | `sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17` |
| 文件 | `model.int8.onnx` + `tokens.txt` |
| 体积 | ~228 MB |
| 来源 | [sherpa-onnx ASR models](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models) |
| 语言 | 中/英/日/韩/粤（Vosi 使用 auto + zh/en/ja） |
| 内置能力 | ITN（数字/日期归一化）、基础标点 |

**选型理由**：

- BPE 词表（25,055 tokens）对英文单词识别优于 Paraformer small
- 原生支持日语，满足三语混说
- 内置 ITN，有机会去掉 280MB 标点模型以缩小安装包
- 仍为 sherpa-onnx `OfflineRecognizer`，无需改流式架构

### 2.2 备选：Paraformer 标准版 INT8

仅当 SenseVoice golden 测试整体不达标时临时降级评估：

| 属性 | 值 |
|------|-----|
| 模型 ID | `paraformer-zh-2024-03-09-int8` |
| 体积 | ~217 MB |
| 语言 | 中英双语（**不支持日语**） |

由于需要日语，Paraformer 不作为最终交付目标。

### 2.3 不变组件

| 组件 | 说明 |
|------|------|
| VAD | Silero `vad/model.onnx`（~2MB），长文模式继续使用 |
| 运行时 | sherpa-onnx 1.13.2 + ONNX Runtime CPU |
| 热词后处理 | `post/hotword.rs` 保留并扩展 |

### 2.4 标点模型：实测后决定

当前标点模型 `punc-ct-transformer`（~280MB）是否保留，由 Phase 1 对比测试决定：

| 条件 | 决策 |
|------|------|
| SenseVoice（`use_itn=true`）的中文标点与 CT-Transformer 相当或更好 | **移除**标点模型及 `PunctuationEngine` 代码路径 |
| SenseVoice 标点明显不足（缺句号/逗号/问号，或英文句标点混乱） | **保留**标点作为后处理步骤 |

判断标准：15 条 golden 样本中，标点正确率 ≥ 90% 则移除；否则保留。

---

## 3. 语言策略

用户场景为**中英日三语混说**（同一句可切换语种）。

### 3.1 默认配置

```toml
[asr]
model_variant = "sense-voice-int8"
language = "auto"    # 自动检测中/英/日
use_itn = true
num_threads = 2
mode = "short"       # short | long
```

- `language = "auto"`：模型自动判断语种，适合三语混说
- v0.1 不做语言手动切换 UI（YAGNI）
- 若 golden 测试中 `auto` 对特定语种不稳定，再考虑暴露高级选项

### 3.2 sherpa-onnx language ID 参考

| 值 | 语种 |
|----|------|
| `auto` (0) | 自动 |
| `zh` (3) | 中文 |
| `en` (4) | 英文 |
| `ja` (11) | 日语 |

---

## 4. 系统架构

### 4.1 推理管线

```
当前：
  音频 → Paraformer ASR → CT-Transformer 标点 → 热词替换 → ITN 规则

目标（SenseVoice，标点达标时）：
  音频 → SenseVoice ASR (auto, use_itn=true) → 热词替换 → [轻量 ITN 补漏]

目标（SenseVoice，标点不达标时）：
  音频 → SenseVoice ASR → CT-Transformer 标点 → 热词替换 → ITN 规则
```

### 4.2 目录结构

```
models/bundled/
├── sense-voice/              # 新 ASR（替换 paraformer-zh/）
│   ├── model.int8.onnx
│   └── tokens.txt
├── vad/model.onnx            # 不变
├── punctuation/              # 实测后可能删除
└── paraformer-zh/            # 测试期间保留备份，验收后删除
    ├── model.int8.onnx
    └── tokens.txt
```

### 4.3 代码改动面

| 模块 | 改动 |
|------|------|
| `asr/engine.rs` | `OfflineParaformerModelConfig` → `OfflineSenseVoiceModelConfig` |
| `asr/paths.rs` | 新增 `resolve_sense_voice_paths()` |
| `asr/model_manager.rs` | 路径解析、版本检测、备份/迁移逻辑 |
| `pipeline/session.rs` | 标点步骤改为可选（配置或 feature flag） |
| `scripts/download-models.sh` | 下载 SenseVoice；测试期保留下载 paraformer-zh |
| `models/manifest.json` | 更新模型清单与版本号 |
| `config/` | 新增 `language`、`use_itn` 字段 |
| `resources/hotwords-tech.txt` | 新增内置技术热词（中/英/日） |

**不改**：`audio/`、`hotkey/`、`inject/`、托盘/UI 交互逻辑。

### 4.4 技术热词包

内置 `resources/hotwords-tech.txt`（约 50–100 条），首次启动合并到用户热词文件（不覆盖已有条目）：

- **程序员（英）**：React, TypeScript, Kubernetes, Docker, GitHub, API, pull request, merge request, npm, webpack…
- **产品经理（英）**：MVP, PRD, KPI, OKR, user story, sprint, backlog, roadmap…
- **日语**：実装, 設計, 要件, バグ, デプロイ, レビュー, スプリント…

热词为后处理兜底，不替代模型本身的识别能力。

---

## 5. 测试验收

### 5.1 Golden 测试集（15 条）

| 类别 | 条数 | 示例期望输出 |
|------|------|-------------|
| 纯中文 | 3 | 「今天开会讨论一下新功能的排期。」 |
| 中文 + 英文技术词 | 4 | 「用 React 实现这个 API 的 pull request。」 |
| 中文 + 日语 | 3 | 「这个功能的実装要用 TypeScript。」 |
| 三语混说 | 3 | 「这个 API の設計 review 一下再 deploy。」 |
| 产品经理场景 | 2 | 「下个 sprint 的 MVP 需求和 KPI 对齐一下。」 |

每条：16kHz mono WAV + 期望文本 JSON。CI 运行：

```bash
cargo test --test asr_golden -- --ignored
```

### 5.2 验收阈值

| 指标 | 阈值 |
|------|------|
| 中文主体准确率 | ≥ 90% |
| 英文技术词正确拼写率 | ≥ 85% |
| 日语词/短语正确输出率 | ≥ 80% |
| 三语混说整句可用率 | ≥ 80% |
| 标点正确率（决定去留标点模型） | ≥ 90% |
| 短句（<5s）推理延迟 | ≤ 800ms |

### 5.3 人工验收

| 场景 | 检查项 |
|------|--------|
| 程序员 | IDE 注释、终端、Git commit message、技术文档 |
| 产品经理 | 飞书/Notion/邮件、需求描述、会议纪要 |
| 三语混说 | 日中英切换自然的句子 |

---

## 6. 实施与迁移计划

### 6.1 阶段

```
Phase 1 — 模型并排评测
  ├─ 下载 SenseVoice INT8（保留 paraformer-zh-small 备份）
  ├─ 录制/合成 15 条 golden WAV
  ├─ 对比 SenseVoice(auto) vs 旧 Paraformer small
  ├─ 对比标点：SenseVoice ITN vs CT-Transformer → 决定去留标点模型
  └─ 决策：SenseVoice 是否达标

Phase 2 — 管线切换
  ├─ 改 AsrEngine → SenseVoice
  ├─ 按 Phase 1 结论处理标点管线
  ├─ 更新 download-models.sh / manifest / bundled
  ├─ 内置技术热词包
  └─ ModelManager 版本检测逻辑

Phase 3 — 验收
  ├─ golden 测试全部通过
  ├─ 程序员/PM/三语混说人工实测
  └─ 确认延迟与体积在可接受范围

Phase 4 — 清理（仅验收通过后执行）
  ├─ 删除 paraformer-zh-small 全部代码引用
  ├─ 删除 models/bundled/paraformer-zh/
  ├─ 若标点已移除：删除 punctuation/ 及 PunctuationEngine
  ├─ 更新 model-list.md、PROJECT-SUMMARY.md、README
  └─ bump manifest.version
```

### 6.2 旧用户迁移

测试期间：

- 新模型安装到 `sense-voice/` 目录，与旧 `paraformer-zh/` 并存
- `ModelManager` 通过 `manifest.version` 判断当前生效模型
- 配置 `model_variant` 切换（开发者/测试用），默认指向 SenseVoice

验收通过后：

- 启动时检测旧 `paraformer-zh/` 目录，删除并写日志
- 用户数据目录自动使用新模型，无需手动操作

### 6.3 安装包体积预算

| 组合 | 预估体积 |
|------|---------|
| SenseVoice + VAD（无标点） | ~230 MB |
| SenseVoice + VAD + 标点 | ~510 MB |
| 测试期含旧模型备份 | +78 MB（临时） |

---

## 7. 错误处理

| 场景 | 处理 |
|------|------|
| SenseVoice 模型文件损坏 | 托盘 Warning，「语音引擎不可用，请重新安装」 |
| 三语混说识别结果为空 | 静默忽略，不上屏 |
| 日语输出为谐音汉字 | 热词兜底；golden 未达标则阻塞发布 |
| ASR 推理超时（>5s） | 取消推理，Toast「识别超时，请重试」 |
| 旧模型备份存在但新模型缺失 | fallback 到旧模型并记录警告（仅测试期） |

---

## 8. 决策记录

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 主选模型 | SenseVoice INT8 | 三语支持、英文 BPE 词表、体积可控 |
| 语言模式 | `auto` | 用户三语混说，无需手动切换 |
| 标点模型 | 实测后决定去留 | SenseVoice ITN 可能替代，以 golden 标点率 ≥ 90% 为界 |
| 旧模型处理 | 测试期保留备份，验收后清理 | 降低迁移风险，用户明确要求 |
| 备选模型 | Paraformer 2024-03-09 INT8 | 仅评估用，不支持日语，非交付目标 |
| 流式改造 | 不做 | 与当前松手离线识别模式不匹配，过度工程 |

---

## 9. 不在范围内

- 流式/在线识别架构改造
- 多模型 UI 切换
- 方言模型
- 语言手动切换设置页（除非 auto 测试不达标）
- 韩语/粤语支持（模型能力具备，但不在本次验收范围）
