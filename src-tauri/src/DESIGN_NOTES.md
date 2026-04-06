# Rust 后端 — Design Notes

本文件记录后端关键架构决策，帮助 agent 和开发者理解「为什么这样设计」。

## Pipeline 状态机

**决策**: 使用显式枚举 `PipelineState` 而非 trait 对象状态模式。

**原因**: 状态数量少且固定 (Idle / Recording / Transcribing / Optimizing / Pasting / Done / Error / Cancelled)，枚举比 trait 更简单，且天然支持 `serde::Serialize` 直接推给前端。

**约束**: `Pipeline` 持有 `AudioRecorder` (Option)，录音时 `Some`，停止后 `None`。不要持有长期引用 —— cpal 的 `Stream` 类型不是 `Send`，必须在创建它的线程上 drop。

## CancellationToken

**决策**: 自建 `CancellationToken` (AtomicBool)，而非用 tokio 的 CancellationToken。

**原因**: Pipeline 的 `cancel()` 从全局快捷键回调（同步上下文）中调用。tokio 的 CancellationToken 需要 async context，不适合。AtomicBool 无锁、无 async 依赖、从任何上下文都能调用。

## spawn_blocking 包裹 CoreAudio

**决策**: `start_recording` 和 `stop_recording` 都通过 `tokio::task::spawn_blocking` 执行。

**原因**: cpal (CoreAudio) 的 Stream 创建/销毁可能阻塞主线程几百毫秒。直接在 async 中执行会阻塞 tokio 的工作线程。

## STT 多后端 trait

**决策**: 定义 `TranscriberBackend` trait，通过 `Box<dyn TranscriberBackend>` 动态分发。

**原因**: whisper-rs 和 sherpa-onnx 是完全不同的 C++ 绑定，API 差异大。trait 抽象让 Pipeline 不关心具体后端。静态分发 (泛型) 会让 Pipeline 变成泛型类型，增加 Tauri managed state 的复杂度。

**扩展方式**: 新增 STT 引擎只需 3 步（见 AGENTS.md Common Tasks）。

## 模型管理与注册表

**决策**: 模型元数据硬编码在 `registry.rs`，不走配置文件。

**原因**: 模型下载 URL、文件校验、后端类型等信息是应用级别的，不应该由用户配置。版本更新时通过代码更新注册表。

存储位置: `~/Library/Application Support/com.input0.dev/models/{model_id}/`

## 错误处理

**决策**: 单一 `AppError` 枚举 + thiserror，通过 `Serialize` 直接传给前端。

**原因**: Tauri commands 需要返回值可序列化。`impl Serialize for AppError` 将错误信息序列化为字符串，前端直接显示。不做错误码体系 — 对于桌面应用来说消息字符串足够。

## panic 防护

**决策**: 全局快捷键回调中使用 `catch_unwind` 包裹。

**原因**: 快捷键回调通过 C FFI 调用（macOS CGEvent），panic 跨 FFI 边界是 UB（undefined behavior）。catch_unwind 确保 panic 被捕获并记录日志，不会导致进程 abort。
