# Vosi 多语言（i18n）设计规格

> **Voice Operational Speech Input** — 中文 / English / 日本語 UI 国际化
>
> 日期：2026-06-08 | 状态：已评审待实施
>
> 前置规格：`docs/specs/2026-06-06-vosi-v01-polish-design.md`

## 1. 背景与目标

### 1.1 动机

Vosi 当前所有用户可见文案均为中文硬编码，分散在前端 HTML/TS 与 Rust 后端（托盘、通知、权限 API）。为支持国际化用户，需增加 **中文（默认）、英文、日文** 三种 UI 语言，并在设置页提供切换入口。

### 1.2 范围

**纳入（用户可见的全部界面）：**

| 模块 | 内容 |
|------|------|
| 设置页 | 标题、副标题、各区块标题、表单标签、选项、按钮、保存状态、权限动态文案 |
| 关于页 | 版本前缀、简介、GitHub 标签 |
| 浮动胶囊 | 「识别中」 |
| 托盘 | 菜单（设置 / 关于 / 退出）+ 三态 tooltip |
| 系统通知 | 麦克风不可用、引擎失败、注入失败、识别超时、剪贴板回退等 |
| 权限 API | 权限名称 / 描述 / 操作按钮、安装进度消息、reinstall 提示 |
| 窗口标题 | 设置窗、关于窗 |

**明确排除：**

- ASR 语音识别语言（`asr.language`，仍为 `auto` 等业务配置，与 UI locale 独立）
- 品牌名 `Vosi`、GitHub URL、热词文件路径
- 硬件键名（Command / Alt / Ctrl）及数值单位 `(ms)`
- 文档站（`docs/`、`README`）的多语言 — 本轮仅应用内 UI
- 基于 OS 系统语言自动选择 UI 语言 — 首次启动固定中文

### 1.3 已确认的产品决策

| 决策 | 选择 |
|------|------|
| 覆盖范围 | 用户可见全部界面（设置 + 关于 + 胶囊 + 托盘 + 通知 + 权限） |
| 生效时机 | 与其他设置项一致，点「保存设置」后生效 |
| 语言选项展示 | 固定原生写法：`中文` / `English` / `日本語` |
| 默认语言 | `zh`（中文） |
| 文案来源 | 开发侧一次性写好三套完整翻译并入库 |

### 1.4 成功标准

- 新用户首次启动界面为中文
- 设置页切换语言并保存后，设置页 / 关于页 / 胶囊 / 托盘菜单与 tooltip 立即更新为目标语言
- 后续系统通知、权限列表 API 返回目标语言文案
- 重启应用后语言偏好持久化
- 三种语言核心 key 无缺失（Rust 单元测试覆盖）

---

## 2. 技术选型

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 整体方案 | 前后端各维护一份 locale JSON | Tauri 分层自然；权限/托盘/通知在 Rust 侧查表；避免跨进程频繁拉字符串 |
| 前端 i18n | 轻量自研 `t()` + `applyLocale()` | ~80 条字符串，无需 i18next |
| Rust i18n | `include_str!` 嵌入 JSON + 启动时 parse 缓存 | 无额外 crate；编译期打包 |
| 配置字段 | `general.locale: "zh" \| "en" \| "ja"` | 与现有 `AppConfig` / `settings.toml` 一致 |
| 切换通知 | Tauri event `locale-changed` | 通知 main / about / overlay 三个 WebView 重渲染 |
| 缺 key fallback | 回退到 `zh` + 开发日志 | 防止空白 UI |

**未选用：**

- **Rust 唯一数据源 + invoke**：权限轮询频繁，跨进程开销大
- **i18next / fluent-rs**：规模过小，YAGNI

---

## 3. 配置与切换流程

### 3.1 存储

```toml
[general]
locale = "zh"   # "zh" | "en" | "ja"，缺省 "zh"
show_tray = true
start_on_boot = true
```

- 旧配置文件无 `locale` 字段时，serde `#[serde(default = "default_locale")]` 返回 `"zh"`
- `locale` 与 `asr.language` 完全独立

### 3.2 设置 UI

- 在「通用」区块**顶部**增加「语言」下拉
- 选项固定显示：`中文` / `English` / `日本語`（value 分别为 `zh` / `en` / `ja`）
- 选中后不立即生效；用户点「保存设置」后与其他项一并持久化

### 3.3 保存后行为（`save_config`）

1. 写入 `settings.toml`
2. 更新 `AppState` 中的当前 `Locale`
3. 若 `locale` 变化，调用 `i18n::apply_locale(app, locale)`：
   - 重建托盘菜单文案
   - 按当前 `TrayStatus` 刷新 tooltip
   - `app.emit("locale-changed", locale)` 广播至所有 WebView
   - 更新 `main` / `about` 窗口标题
4. 前端 `save_config` 成功回调中也调用 `applyLocale()`（双保险，防事件丢失）

### 3.4 首次启动

- 始终使用 `zh`，不读取 OS 系统语言

---

## 4. 文件结构

```
src/
  i18n/
    index.ts          # t(key, vars?)、applyLocale()、setLocale()、getLocale()
    types.ts          # Locale = "zh" | "en" | "ja"
  locales/
    zh.json
    en.json
    ja.json

src-tauri/src/
  i18n/
    mod.rs            # t(locale, key) -> String；apply_locale(app, locale)
    locale.rs         # Locale 枚举、from_str、default
  locales/
    zh.json
    en.json
    ja.json
```

### 4.1 Key 命名约定

点分层级，前后端语义对齐（各自维护翻译值）：

```
settings.title
settings.subtitle
settings.section.general
settings.field.locale
settings.option.locale.zh          # 下拉显示「中文」（三语言固定原生写法，三份 JSON 值相同）
tray.menu.settings
tray.tooltip.idle
notify.mic_unavailable
perm.microphone.label
perm.microphone.action.request
overlay.processing
about.version_prefix
about.description
window.main.title
window.about.title
```

- 含变量：`"voice.status.installing": "语音功能：{message}"`
- 缺 key：fallback `zh`；开发模式 `console.warn` / `eprintln!`

### 4.2 HTML 改造

- `index.html` / `about.html`：移除硬编码中文，改用 `data-i18n="key"` 标记静态文本
- `applyLocale()` 扫描 `[data-i18n]` 并更新 `textContent`
- 带占位符的元素使用 `data-i18n-vars='{"name":"value"}'`
- 动态内容（权限列表、保存状态、setup 消息）在 TS / Rust 运行时调用 `t()`

---

## 5. Rust 集成

### 5.1 Locale 生命周期

- `AppState` 持有当前 `Locale`（启动时从 `AppConfig.general.locale` 初始化）
- 所有需要本地化字符串的路径从 `AppState` 或显式 `Locale` 参数获取

### 5.2 需改造的 Rust 模块

| 模块 | 改造 |
|------|------|
| `config/types.rs` | `GeneralConfig` 增加 `locale: String`，默认 `"zh"` |
| `app/state.rs` | 缓存当前 `Locale`；`set_config` 时同步更新 |
| `app/tray.rs` | `tooltip_for(status, locale)`；菜单文案参数化 |
| `lib.rs` | `setup_tray_menu(app, locale)`；setup 消息、notifier 改用 `t()` |
| `permissions/macos.rs` | `snapshot(state, locale)` 返回翻译后的权限字段 |
| `commands.rs` | `get_app_info(locale)` 或从 state 读 locale 返回本地化 description |
| `commands.rs` | `save_config` 末尾触发 `apply_locale` |

### 5.3 `i18n::apply_locale` 职责

```text
apply_locale(app, locale):
  1. rebuild tray menu (settings / about / quit)
  2. refresh tray tooltip for current TrayStatus
  3. emit "locale-changed" with locale string
  4. set window titles for "main" and "about"
```

### 5.4 Rust i18n 实现要点

- 三份 JSON 通过 `include_str!` 嵌入，首次调用时 parse 到 `LazyLock<HashMap<Locale, HashMap<String, String>>>`
- `t(locale, key)` 支持 `{var}` 简单替换
- 不引入 `rust-i18n` / `fluent` crate

---

## 6. 前端数据流

```text
启动:
  invoke get_config
  → setLocale(cfg.general.locale)
  → applyLocale()
  → 渲染权限列表（后端 snapshot 已按 locale 返回）

保存:
  invoke save_config
  → 后端 apply_locale + emit locale-changed
  → 前端 save 回调 applyLocale()
  → 重刷权限列表

locale-changed 事件:
  main.ts / about.ts / overlay.ts 监听
  → applyLocale() + 必要时重渲染动态区块
```

### 6.1 各 WebView 职责

| 文件 | 改造 |
|------|------|
| `main.ts` | 初始化 locale；监听事件；权限 / 保存状态 / voice status 用 `t()` |
| `about.ts` | 版本前缀、description、GitHub 标签 |
| `overlay.ts` | 启动读 config 取 locale；监听 `locale-changed` 更新「识别中」 |
| `index.html` | `data-i18n` 标记全部静态文案 |
| `about.html` | 同上 |

---

## 7. 错误处理

| 场景 | 行为 |
|------|------|
| `settings.toml` 中非法 locale 值 | 解析失败回退 `zh` |
| JSON key 缺失 | fallback 到 `zh` 对应 key；打 warn 日志 |
| `locale-changed` 事件未送达 | 前端 `save_config` 成功回调仍执行 `applyLocale()` |
| 权限 snapshot 在语言切换后 | 前端收到事件后重新 `loadPermissions()` |

---

## 8. 测试策略

### 8.1 Rust 单元测试

- `Locale::from_str` 合法 / 非法输入
- `default_locale()` 为 `"zh"`
- 三种 locale 下核心 key 存在：`tray.menu.*`、`notify.*`、`perm.microphone.label`
- `GeneralConfig` 反序列化缺字段时使用默认 locale
- `save_config` 切换 locale 后 `AppState` 反映新值

### 8.2 手动验收清单

- [ ] 首次安装 / 无 locale 字段的旧配置 → 界面中文
- [ ] 设置 → 语言选 English → 保存 → 设置页全英文
- [ ] 托盘菜单、tooltip 同步英文
- [ ] 打开关于页 → 英文
- [ ] 按住热键 → 胶囊「Processing…」英文
- [ ] 触发错误通知（如无麦克风）→ 英文
- [ ] 权限区块 label / 按钮英文
- [ ] 切换日文重复上述步骤
- [ ] 重启应用 → 语言保持
- [ ] 语言下拉始终显示 `中文` / `English` / `日本語`（不随当前 UI 语言变化）

---

## 9. 文案量级估算

| 层 | 约计 key 数 |
|----|------------|
| 前端 settings + about | ~45 |
| Rust tray + notify + perm + setup | ~35 |
| **合计** | **~80** |

三套语言均由开发侧编写，日文/英文为自然表达（非机翻直出），后续可通过 PR 润色。

---

## 10. 不在本轮的后续增强

- OS 语言自动检测（首次启动跟随系统）
- 语言切换无需保存即预览
- 文档站 i18n
- Windows 权限模块独立文案（当前权限 snapshot 主要为 macOS；Windows 扩展时沿用同一 key 体系）
