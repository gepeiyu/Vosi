# 待完成：开机自启平台注册

> **状态：** 未开始  
> **创建：** 2026-06-06  
> **优先级：** 中（不影响日常 `tauri dev` 与语音输入核心功能）  
> **关联规格：** `docs/specs/2026-06-06-vosi-v01-polish-design.md` §7.3

---

## 背景

v0.1.1 已在设置页和配置文件中支持「开机自启动」**开关与持久化**，但切换开关**不会**在操作系统层注册或取消登录启动项。

用户勾选后，仅写入：

```toml
# ~/.config/vosi/settings.toml（macOS/Linux）
# 或 %APPDATA%\Vosi\settings.toml（Windows）

[general]
start_on_boot = true   # 或 false
```

设置页备注：「平台注册即将支持」（见 `index.html`）。

---

## 已完成（无需重复做）

| 项 | 位置 |
|----|------|
| 配置字段 `general.start_on_boot` | `src-tauri/src/config/types.rs` |
| 加载/保存 `settings.toml` | `src-tauri/src/config/mod.rs` |
| 设置 UI 复选框 `#start-on-boot` | `index.html`、`src/main.ts` |

---

## 待实现

### 目标行为

| 用户操作 | 期望结果 |
|----------|----------|
| 开启「开机自启动」并保存 | 下次登录系统自动启动 Vosi（托盘常驻，无主窗口） |
| 关闭并保存 | 移除登录启动项，不再自动启动 |
| 应用卸载 / 退出 | 启动项被清理，不留孤儿 plist/注册表项 |

### macOS 方案（推荐路径）

**选项 A — LaunchAgent plist（传统，可控）**

- 写入：`~/Library/LaunchAgents/com.vosi.app.plist`
- `ProgramArguments` 指向 `.app` 内可执行文件或 `open -a Vosi`
- `RunAtLoad` = true；`KeepAlive` = false（仅需登录启动一次）
- 关闭自启：删除 plist，`launchctl bootout` / `unload`

**选项 B — SMAppService / Login Items API（macOS 13+，更现代）**

- 需评估 Tauri 打包后的 `.app` bundle id 是否与 `com.vosi.app` 一致
- 用户体验更好（系统设置里可见登录项）

**注意：**

- Debug 构建路径与 Release `.app` 不同，应对 `cfg(debug_assertions)` 区分或仅在 Release 启用
- 需处理用户手动在系统设置里删除登录项后，与 `settings.toml` 状态同步（可选：启动时检测并回写配置）

### Windows 方案

**注册表 Run 键（用户级，无需管理员）**

- 键：`HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`
- 值名：`Vosi`（或与 `productName` 一致）
- 值：Release 安装后的 `vosi.exe` 完整路径（带引号）

关闭自启：删除该 Run 值。

**注意：**

- NSIS 安装路径需稳定（如 `%LOCALAPPDATA%\Programs\Vosi\vosi.exe`）
- 卸载程序应一并清理 Run 键

---

## 建议实现结构

```
src-tauri/src/autostart/
├── mod.rs          # pub fn apply(config: &AppConfig) -> Result<(), String>
├── macos.rs        # LaunchAgent 或 SMAppService
└── windows.rs      # RegSetValueEx / winreg crate
```

**调用时机：**

1. **`save_config` IPC** — 用户保存设置后立即 `autostart::sync(config.general.start_on_boot)`
2. **（可选）应用启动** — `setup()` 中若磁盘配置与平台状态不一致，以配置为准修复

**新增 Tauri command（可选）：**

- `sync_autostart` — 供设置页保存后调用，或合并进现有 `save_config`

---

## 实现检查清单

- [ ] macOS：开启自启后，注销/重启后 Vosi 托盘出现
- [ ] macOS：关闭自启后，plist 已删除且不再自动启动
- [ ] Windows：开启自启后，重启后进程存在
- [ ] Windows：关闭自启后，Run 键已删除
- [ ] 卸载应用后无残留 LaunchAgent / Run 项
- [ ] Debug 模式行为明确（禁用或写 dev 路径并文档说明）
- [ ] 更新 `docs/guides/manual-test-checklist.md` 增加自启验收项
- [ ] 更新 `index.html` 备注（移除「即将支持」或改为平台说明）

---

## 依赖与风险

| 风险 | 缓解 |
|------|------|
| 可执行路径随安装位置变化 | 启动时用 `std::env::current_exe` 或 Tauri `path().executable_dir()` |
| macOS 沙盒 / 签名 | Release 需正确 codesign；LaunchAgent 指向 signed `.app` |
| 与 Tauri 单实例冲突 | 确认 `tauri.conf.json` 是否需 `single_instance` 插件 |
| 用户无权限写 LaunchAgents | 仅用户级路径，一般无问题 |

**可选 Rust 依赖：**

- Windows：`winreg`
- macOS：纯 `std::fs` + plist 序列化，或 `plist` crate

---

## 参考

- 配置默认值：`src-tauri/src/config/types.rs` → `GeneralConfig.start_on_boot`
- Tauri identifier：`com.vosi.app`（`tauri.conf.json`）
- v0.1.1 设计决策：`docs/specs/2026-06-06-vosi-v01-polish-design.md` §7.3、§10 决策表「开机自启」

---

## 完成后

- [ ] 将本文档顶部 **状态** 改为「已完成」，并写上完成日期与 commit/PR 链接
- [ ] 或删除本文档，改在 CHANGELOG / README 中记录（二选一即可）
