# Feature: 单键长按触发录音（Single-Key Push-to-Talk）

**状态**: 待开始
**创建日期**: 2026-04-25
**分支**: `feat/single-key-hotkey`

## 背景与需求

当前录音快捷键只能是组合键（如 `Option+Space`），由 `tauri-plugin-global-shortcut` 负责。项目已对 `Fn` 做了特殊处理，通过 `NSEvent.flagsChanged` 实现单键长按。用户反馈希望支持更多的单键方案，例如"右 Option"。

目标：让用户可以把**任意单个修饰键**（含左右分离）设置为录音快捷键，并且按下时该键默认的系统行为被**消费**（不再触发重音输入、Spotlight 等）。

## 为什么标准热键 API 不够

- `tauri-plugin-global-shortcut` → 底层 `global-hotkey` crate → macOS Carbon `RegisterEventHotKey`。该 API **要求热键至少包含一个非修饰主键**，所以无法注册"单个 Option"这类热键。
- 现有 `fn_monitor.rs` 用 `NSEvent addGlobalMonitorForEventsMatchingMask` 订阅 `flagsChanged`，但 **NSEvent 全局监听器无法消费事件**——只能"旁听"，按键默认行为照样会发送给前台 app。

需求明确要"消费"，因此必须切换到更底层的 **CGEventTap (Quartz Event Services)**。

## 技术方案

### 架构：`fn_monitor.rs` 统一重构为 `single_key_monitor.rs`

原 NSEvent 实现全部迁移到 CGEventTap。Fn 键也走新路径——用户已确认 Fn 被消费是可接受的。

```rust
// src-tauri/src/input/single_key_monitor.rs
pub enum SingleKey {
    Fn,
    LeftOption,    RightOption,
    LeftCommand,   RightCommand,
    LeftControl,   RightControl,
    LeftShift,     RightShift,
}

pub fn start(key: SingleKey, callback: Arc<dyn Fn(bool) + Send + Sync + 'static>) -> Result<(), String>;
pub fn stop() -> Result<(), String>;
```

### 事件识别

订阅 `CGEventMaskBit(kCGEventFlagsChanged)`；回调中读取 `kCGKeyboardEventKeycode` 区分左右修饰键：

| Key | keyCode |
|---|---|
| Fn | 63 |
| Left Option / Right Option | 58 / 61 |
| Left Command / Right Command | 55 / 54 |
| Left Control / Right Control | 59 / 62 |
| Left Shift / Right Shift | 56 / 60 |

维护每个目标键的 `AtomicBool` 状态，仅在状态**翻转**（按下 / 松开）时派发 callback，避免重复触发。

### 事件消费

Tap 回调返回约定：

- 目标键状态翻转 → 调用业务 callback；返回 `NULL`（吞掉事件，其他 app 看不到这次 flagsChanged）
- 其他修饰键或目标键无状态变化 → 返回原 `event`（透传）

**副作用（需在 UI 警示）**：
- Right Option → 吞掉重音输入（`⌥E` → é 等）
- Right Command / Left Command → 该键单独按下的系统行为被吞。**警告**：组合键 `Cmd+C/V` 在"按住 Cmd 作为 push-to-talk"期间**极可能失效**，因为下游 app 不再知道 Cmd 已按下。实现时必须手动验证这一点——如果确实失效，则 Cmd 类单键不推荐作为默认选项，并在 UI 副作用提示中明示
- Fn → 亮度 / 音量等功能行按键会被吞
- 任意修饰键 → 该键参与的组合键（如 `⌥+Tab` 这种）在按下阶段可能受影响，需要手动验证

### 线程模型

CGEventTap 必须挂在 `CFRunLoop`。方案：

1. `start` 时创建专用后台线程，在其中：
   - `CGEventTapCreate(kCGSessionEventTap, kCGHeadInsertEventTap, kCGEventTapOptionDefault, mask, callback, user_info)`
   - `CFMachPortCreateRunLoopSource` → `CFRunLoopAddSource` → `CFRunLoopRun`
2. `stop` 时：
   - `CGEventTapEnable(tap, false)` + `CFMachPortInvalidate(tap)`
   - `CFRunLoopStop(run_loop)` → `join` 线程

线程安全：callback 是 `Arc<dyn Fn + Send + Sync>`，状态用 `AtomicBool` + `Mutex<Option<TapHandles>>`（和现有 `fn_monitor.rs` 的模式一致）。

### 权限

- **Input Monitoring**（已有，Fn 监听就已经要求）
- **Accessibility**（新增，CGEventTap 消费事件必需）

首次 `CGEventTapCreate` 返回 NULL → 说明未授予 Accessibility 权限 → `start` 返回 `Err("Accessibility permission required")`。前端捕获此错误时显示横幅并提供跳转系统设置的按钮。

### `lib.rs` 路由改造

```rust
// 替换 is_fn_hotkey
pub fn parse_single_key(raw: &str) -> Option<SingleKey>;

pub fn register_pipeline_shortcut(app: &AppHandle, raw: &str) -> Result<()> {
    if let Some(sk) = parse_single_key(raw) {
        single_key_monitor::start(sk, move |pressed| {
            if pressed { trigger_pipeline_pressed(&app_clone) }
            else       { trigger_pipeline_released(&app_clone) }
        })
    } else {
        // 原 global-shortcut 组合键路径（不变）
    }
}
```

### 配置字符串约定

向后兼容，扩展现有 raw 字符串集：

| Raw | 含义 |
|---|---|
| `"Fn"` | Fn 键（保留现有兼容） |
| `"RightOption"` / `"LeftOption"` | 右 / 左 Option |
| `"RightCommand"` / `"LeftCommand"` | 右 / 左 Command |
| `"RightControl"` / `"LeftControl"` | 右 / 左 Control |
| `"RightShift"` / `"LeftShift"` | 右 / 左 Shift |
| `"Option+Space"` 等 | 原有组合键格式不变 |

**默认热键不变**（保持现有配置）。

### 前端改动

`src/components/SettingsPage.tsx`：

1. 热键输入区新增"类型"切换：
   - **组合键**：保持现有输入框
   - **单键**：下拉菜单列出全部 9 个单键选项
2. 选择单键后显示对应副作用提示，例如：
   > ⚠️ 选择"右 Option"后，将无法用它输入重音字符（如 ⌥E → é）。
3. 若启动单键监听失败（权限缺失），顶部显示横幅 + "打开系统设置"按钮。

### 测试计划

**单元测试**（`src-tauri/src/input/single_key_monitor.rs` 内 `#[cfg(test)]`）：
- `parse_single_key` 的所有 variant + 无效输入 + 大小写不敏感
- 状态翻转逻辑：模拟 flagsChanged 序列，断言仅在状态变化时触发回调，且每次 press 对应一次 release

**手动验证**（无法自动化）：
- 9 个单键逐个设置为热键：在前台 / 后台 app 下按住 → 录音开始；松开 → 录音结束
- 验证消费：选右 Option 后，打开 TextEdit 按 `⌥E` 应不输入 é
- 验证权限：首次运行时无 Accessibility 权限 → 前端横幅正确显示
- 验证切换：组合键 ↔ 单键切换后配置持久化正常

## 文件清单

| 文件 | 变动类型 |
|---|---|
| `src-tauri/src/input/fn_monitor.rs` | **重命名** → `single_key_monitor.rs`，重写为 CGEventTap |
| `src-tauri/src/input/mod.rs` | 更新模块导出 |
| `src-tauri/src/lib.rs` | `is_fn_hotkey` → `parse_single_key`；`register/unregister_pipeline_shortcut` 路由 |
| `src-tauri/src/input/hotkey.rs` | 添加 `parse_single_key(raw) -> Option<SingleKey>`；保持 `parse_hotkey` / `to_tauri_shortcut` 原语义不变（组合键路径用） |
| `src-tauri/src/commands/input.rs` | 扩展 hotkey 验证；新增 `open_accessibility_settings` 命令 |
| `src-tauri/Cargo.toml` | 新增 `core-foundation` / `core-graphics` 依赖（CGEventTap FFI） |
| `src/components/SettingsPage.tsx` | 热键选择器 UI + 副作用提示 + 权限横幅 |
| `src/stores/settings-store.ts` | 无 schema 变更（仍是字符串） |
| `docs/feature-single-key-hotkey.md` | 本文档 |

## 回滚策略

CGEventTap 在部分机型或特定 macOS 版本可能行为异常（历史上 macOS Big Sur 早期版本有权限弹窗 bug）。保留编译特性 flag `single_key_nsevent_fallback`，需要时可回退到 NSEvent（不消费）模式作为保险。默认不启用，仅作为问题发生时的快速降级手段。

## 实现状态

- [ ] CGEventTap 后端实现
- [ ] `parse_single_key` 及单元测试
- [ ] `lib.rs` 路由改造
- [ ] 前端 Settings UI
- [ ] 权限横幅与跳转系统设置
- [ ] 手动回归测试 9 个单键
- [ ] 更新 CLAUDE.md 索引表
