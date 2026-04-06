# Feature: Voice Input Pipeline + App Startup Flow

## 状态：已完成 ✅

## 实现内容

### Part A: `src-tauri/src/pipeline.rs`

核心状态机，管理录音→转录→优化→粘贴的完整流程。

**关键设计决策**：

- `cpal::Stream` 不实现 `Send`，导致 `Pipeline` 也不是 `Send`。通过 `unsafe impl Send + Sync for Pipeline` 解决，Mutex 保证访问安全性。
- `stop_and_process` 拆分为两步，避免在 async fn 中跨 await 持有非 `Send` 的 `AudioRecorder`：
  - `stop_recording_sync(&mut self) -> Result<RecordedAudio>` — 同步提取录音数据
  - `process_audio(recorded: RecordedAudio, app: AppHandle) -> Result<String>` — 异步处理（转录→优化→粘贴）
- `ManagedPipeline = Arc<Mutex<Pipeline>>` 作为 Tauri state 类型

**状态机事件**（通过 `app.emit("pipeline-state", PipelineEvent { state })` 通知前端）：

```
Idle → Recording → Transcribing → Optimizing → Pasting → Done { text }
                                                         → Error { message }
```

### Part B: `src-tauri/src/commands/window.rs`

窗口管理命令：
- `show_overlay` — 显示 overlay 窗口
- `hide_overlay` — 隐藏 overlay 窗口
- `show_settings` — 显示并聚焦 main 窗口

### Part C: `src-tauri/src/commands/audio.rs`

Pipeline 控制命令：
- `start_recording` — 开始录音
- `stop_recording` — 停止录音并处理（返回最终文本）
- `toggle_recording` — 切换录音状态（返回当前是否录音中）

### Part D: `src-tauri/src/lib.rs` 更新

Setup 逻辑：
1. 加载配置 (`config::load`)
2. 从 resource_dir 或 `config.model_path` 加载 Whisper 模型
3. 注册全局快捷键（按下时切换录音状态）
4. 管理 `Arc<Mutex<Pipeline>>` 作为 Tauri state

快捷键处理：使用 `tauri_plugin_global_shortcut::GlobalShortcutExt` 的 `on_shortcut`，在 `ShortcutState::Pressed` 事件触发时：
- 若未录音：同步调用 `start_recording`
- 若录音中：同步调用 `stop_recording_sync` 获取数据，再 `tokio::spawn` 异步处理

## 验证结果

- `cargo test --lib`: 89 passed, 0 failed, 8 ignored ✅
- `cargo build`: Finished ✅
