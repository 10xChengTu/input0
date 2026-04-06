# 活跃应用名称作为 LLM 领域信号

## 状态：已完成 ✅

## 需求描述

在用户按下录音快捷键时，获取当前前台活跃应用的名称（如 VS Code、Slack、Notes），将其作为轻量级领域信号传入 LLM 文本优化请求。LLM 可据此推断语境（技术/聊天/写作），提升语音转文字后的纠错质量。

**设计选择**：最初考虑将剪贴板最近 5 条内容作为上下文，但剪贴板数据量大、隐私敏感。最终选择仅捕获应用名称——信息密度高、零隐私风险、几乎无性能开销。

## 技术方案

### 前台应用名称获取

通过 macOS `NSWorkspace` API 同步获取当前前台应用的 `localizedName`：

```rust
// input/mod.rs
pub fn get_frontmost_app() -> Option<String>
```

- 使用 `objc` + `cocoa` crate（项目已有依赖）
- `NSWorkspace.sharedWorkspace.frontmostApplication.localizedName`
- 非 macOS 平台返回 `None`（优雅降级）
- 任何 API 失败均返回 `None`，不影响主流程

### 关键时序：在 Overlay 显示前捕获

```
用户按下快捷键 → get_frontmost_app()（此时前台还是用户应用）
  → Pipeline.set_source_app(app_name)
  → spawn async { 显示 Overlay 窗口 }（此时前台变为 Input0）
```

必须在 overlay 窗口显示前同步调用 `get_frontmost_app()`，否则捕获到的前台应用会是 Input0 自身。

### Pipeline 数据流

```
lib.rs Pressed handler
  → get_frontmost_app() → Pipeline.set_source_app()
  → 录音开始...
  → stop_recording_sync() 将 source_app 移入 RecordedAudio
  → process_audio() 读取 recorded.source_app.as_deref()
  → optimize_text(..., source_app)
  → build_context_message(history, source_app)
  → LLM context message: "[Active application: VS Code]"
```

- `Pipeline` 新增 `source_app: Option<String>` 字段 + `set_source_app()` 方法
- `RecordedAudio` 新增 `source_app: Option<String>` 字段
- `stop_recording_sync()` 中 `self.source_app.take()` 移入 `RecordedAudio`

### LLM 集成

**`optimize_text()` 签名变更**：新增第 6 个参数 `source_app: Option<&str>`

**`build_context_message()` 变更**：
- 新增 `source_app: Option<&str>` 参数
- 当有 source_app 时，即使无历史记录也返回 `Some`（context 消息）
- 格式：`[Active application: {name}]\n\n` 前置于历史上下文

**System prompt 更新**：在 `context_instructions` 中新增活跃应用使用指引：
- IDE/编辑器 → 偏向技术术语纠错
- 聊天/即时通讯 → 保留口语风格
- 写作/笔记 → 偏向正式措辞
- 仅作为辅助信号，不应过度依赖

### 降级策略

所有环节 `source_app` 均为 `Option`：
- 非 macOS 平台 → `None`
- API 调用失败 → `None`
- Pipeline 中未设置 → `None`
- IPC 直接调用（非快捷键触发） → `None`

任何情况下 `None` 不影响现有功能，LLM 仍正常工作。

## 改动文件

| 文件 | 改动 |
|------|------|
| `src-tauri/src/input/mod.rs` | 新增 `get_frontmost_app()` 函数，通过 NSWorkspace API 获取前台应用名 |
| `src-tauri/src/pipeline.rs` | `Pipeline` 新增 `source_app` 字段 + `set_source_app()` 方法；`RecordedAudio` 新增 `source_app` 字段；`stop_recording_sync()` 移交 source_app；`process_audio()` 传递 source_app |
| `src-tauri/src/llm/client.rs` | `optimize_text()` 新增 `source_app` 参数；`build_context_message()` 新增 `source_app` 参数，支持单独 source_app 或与历史上下文组合；system prompt 新增领域信号使用指引 |
| `src-tauri/src/lib.rs` | `ShortcutState::Pressed` handler 中调用 `get_frontmost_app()` 并传入 Pipeline |
| `src-tauri/src/commands/llm.rs` | IPC `optimize_text` 调用传入 `None` 作为 source_app（IPC 路径不提供应用上下文） |
| `src-tauri/src/llm/tests.rs` | 所有 21 个 `optimize_text` 调用 + 3 个 `build_context_message` 调用更新参数；新增 4 个测试覆盖 source_app 场景 |

## 新增测试

| 测试 | 覆盖场景 |
|------|----------|
| `test_build_context_message_with_source_app_only` | 仅有 source_app、无历史时生成 context 消息 |
| `test_build_context_message_with_source_app_and_history` | source_app + 历史记录组合 |
| `test_build_context_message_no_app_no_history` | 两者均无时返回 None |
| `test_system_prompt_contains_active_app_instructions` | system prompt 包含活跃应用使用指引 |

## 验证结果

- `cargo test --lib`: 139 passed, 0 failed, 8 ignored ✅
- `pnpm build`: TypeScript 类型检查 + Vite 构建成功 ✅
- 编译 warnings: 7 个 `cocoa::base::{id, nil}` deprecation 警告（预存问题，非本次引入）

## 后续优化方向

1. **窗口标题捕获** — 获取活跃窗口的标题（如文件名、频道名）提供更精确的语境信号
2. **应用→领域映射表** — 维护常见应用到领域的预设映射，减轻 LLM 推断负担
3. **迁移到 objc2 crate** — 消除 `cocoa::base::{id, nil}` deprecation 警告
