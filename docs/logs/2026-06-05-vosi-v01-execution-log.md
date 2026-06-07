# 执行日志 - Vosi v0.1

## 元信息
- 开始时间: 2026-06-05T14:10:00+08:00
- 执行模式: Subagent-Driven
- GitHub: https://github.com/gepeiyu/Vosi.git
- 规格: `docs/specs/2026-06-05-vosi-v01-design.md`
- 计划: `docs/plans/2026-06-05-vosi-v01.md`

## 任务执行记录

### Task 1–12 — ✅ Completed
- 详见上文各 Task 条目

### Task 13: Privacy-Safe Logging — ✅ Completed
- 实现: `src-tauri/src/log/mod.rs`
- 特性: 文件追加、1MB 轮转、推理元数据日志（不含识别文本）
- 测试: `logger_writes_to_file` PASS

### Task 14: Golden Audio Integration Tests — ✅ Completed
- 实现: `src-tauri/tests/asr_golden.rs`（5 个 `#[ignore]` 测试）
- 文档: `tests/fixtures/audio/README.md`, `docs/guides/manual-test-checklist.md`
- 注: WAV 样例需本地录制后放入 fixtures 目录

### Task 15: CI and Release — ✅ Completed
- 实现: `.github/workflows/ci.yml`, `.github/workflows/release.yml`
- 脚本: `scripts/prepare-bundle-models.sh`
- CI: 单元测试 + clippy + 离线运行时依赖检查

### Task 16: Documentation — ✅ Completed
- 实现: `docs/guides/quick-start.md`, `docs/guides/model-list.md`
- 更新: `README.md`, `README.zh-CN.md`

## 验证

```bash
export SHERPA_ONNX_ARCHIVE_DIR=/Users/silverwing/develop/Vosi/.cache/sherpa-onnx
export VOSI_PROXY=http://127.0.0.1:7890   # 可选，加速 HuggingFace/GitHub
./scripts/download-models.sh

cd src-tauri && cargo test --lib
# 2026-06-06: 11/11 PASS

cargo test --test asr_pipeline -- --ignored
cargo test --test asr_golden -- golden_short_greeting --ignored
# 2026-06-06: PASS（sherpa test_wavs/0.wav 作占位 fixture）

npm run tauri dev
# 2026-06-06: voice session initialized（debug 自动从 models/dev 安装）
```

## 收尾记录（2026-06-06）

- **模型镜像**：魔搭 FunASR ONNX 与 sherpa-onnx 不兼容；ASR/标点须 csukuangfj 预打包格式
- **下载脚本**：`VOSI_PROXY` + `hf-mirror` / HuggingFace 双通道
- **开发体验**：debug 构建自动将 `models/dev/` 复制到 `~/Library/Application Support/vosi/models/`
- **默认热键**：macOS 右 Command / Windows 右 Alt（commit `360b08a`）
- **macOS E2E**：用户确认按住说话 + 文本注入正常（2026-06-06）
- **GitHub 开源**：`main` 推送 + tag [v0.1.0](https://github.com/gepeiyu/Vosi/releases/tag/v0.1.0)（2026-06-06）

## v0.1 后续（2026-06-07）

v0.1.1 polish 在 `feat/v0.1.1-polish` 分支继续，详见：

- [2026-06-07-v01-polish-wrap-up.md](2026-06-07-v01-polish-wrap-up.md)
- 总览：[../PROJECT-SUMMARY.md](../PROJECT-SUMMARY.md)

## v0.1 全部 Task 完成

```
Task 1–16 ✅
收尾验证 ✅（ASR 集成测试 + macOS E2E + GitHub v0.1.0 开源）
```
