# Golden Audio Fixtures

Place **16 kHz mono 16-bit PCM WAV** files here for SenseVoice integration tests. Each file should be under 1 MB.

Fixtures are not committed by default (large binary). Copy your recordings into this directory before running golden tests.

Expectations for each case are defined in `golden.json` (`must_contain` / `must_not_contain`).

## Recording scripts (15 cases)

Record the following phrases in a quiet room. Export as 16 kHz mono WAV using the filename shown.

| File | Category | Script |
|------|----------|--------|
| `zh_pure_1.wav` | 纯中文 | 今天开会讨论一下新功能的排期。 |
| `zh_pure_2.wav` | 纯中文 | 这个功能的实现方案需要评审。 |
| `zh_pure_3.wav` | 纯中文 | 请完成单元测试和集成测试。 |
| `en_tech_react.wav` | 英文技术词 | 用 React 实现这个 API 的接口 |
| `en_tech_ts.wav` | 英文技术词 | 用 TypeScript 重写这个组件的逻辑 |
| `en_tech_pr.wav` | 英文技术词 | 请帮我提交一个 pull request |
| `en_tech_k8s.wav` | 英文技术词 | 把这个服务部署到 Kubernetes 集群 |
| `ja_mixed_impl.wav` | 中日混说 | 这个功能的実装需要用 TypeScript |
| `ja_mixed_design.wav` | 中日混说 | 先看一下这个模块的設計文档 |
| `ja_mixed_bug.wav` | 中日混说 | 线上有个バグ需要紧急修复 |
| `trilingual_1.wav` | 三语混说 | 这个 API の設計 review 一下再 deploy |
| `trilingual_2.wav` | 三语混说 | 先在 staging 环境 deploy 一下 |
| `trilingual_3.wav` | 三语混说 | 请 review 一下这个方案的 feasibility |
| `pm_mvp.wav` | 产品经理 | 下个 sprint 的 MVP 需求和 KPI 对齐一下 |
| `pm_sprint.wav` | 产品经理 | 这个 sprint 的目标是完成 backlog 梳理 |

## Recording tips

1. Sample rate **16 kHz**, **mono**, 16-bit PCM.
2. Record in a quiet room; avoid background music and fan noise.
3. Speak naturally at normal pace — these cases cover programmer and PM dictation.

## Run tests

Download models first, then run the data-driven golden test:

```bash
./scripts/download-models.sh
cd src-tauri
cargo test --test asr_golden golden_all_cases -- --ignored --nocapture
```

Missing WAV files are skipped with a message; at least one fixture must exist or the test fails.
