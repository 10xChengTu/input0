# Feature: ESC 键取消语音输入

## 状态：已完成 ✅

## 需求

用户在语音输入过程中（无论处于 Recording / Transcribing / Optimizing / Pasting 任何阶段），按下 ESC 键即可立即取消当前操作，不会将文字粘贴到输入框。

## 实现方案

### 取消机制：CancellationToken

在 `pipeline.rs` 中新增 `CancellationToken`（基于 `Arc<AtomicBool>`），嵌入 `Pipeline` struct：

- `start_recording` 时自动 `reset()` token
- `cancel()` 时设置 flag、停止录音、emit `Cancelled` 状态
- `process_audio` 在每个关键步骤前检查 `is_cancelled()`，若已取消则提前返回

### 新增 PipelineState 枚举值

```rust
Cancelled  // 用户主动取消
```

### ESC 全局快捷键（lib.rs）

通过 `tauri_plugin_global_shortcut` 注册 `Escape` 快捷键：

- 仅响应 `ShortcutState::Pressed`
- 调用 `Pipeline::cancel()` 设置取消标志 + 停止录音
- 隐藏 overlay 窗口

### process_audio 取消检查点

在以下位置插入 `cancel.is_cancelled()` 检查：

1. 函数入口
2. 音频转换后、转录前
3. `spawn_blocking` 内转录前
4. 转录完成后
5. LLM 优化完成后
6. 粘贴前

### 前端处理（useTauriEvents.ts）

`cancelled` 状态映射到 `reset()`，Overlay 立即隐藏。

### 新增 Tauri Command

`cancel_pipeline` — 供前端在需要时主动调用取消。

## 涉及文件

| 文件 | 变更 |
|------|------|
| `src-tauri/src/pipeline.rs` | 新增 `CancellationToken`、`Cancelled` 状态、`cancel()` 方法、`process_audio` 取消检查 |
| `src-tauri/src/commands/audio.rs` | 新增 `cancel_pipeline` command，`stop_recording`/`toggle_recording` 传入 cancel token |
| `src-tauri/src/lib.rs` | 注册 Escape 全局快捷键、注册 `cancel_pipeline` command、Released 分支传入 cancel token |
| `src/hooks/useTauriEvents.ts` | 处理 `cancelled` pipeline 状态 |
