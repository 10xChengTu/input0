# Architecture

## 系统概览

Input0 是一个 Tauri v2 桌面应用（macOS），采用双进程架构：

```
┌─────────────────────────────────────────────────────┐
│                    macOS 应用                         │
│                                                      │
│  ┌──────────────────┐    ┌───────────────────────┐  │
│  │   React 前端      │    │     Rust 后端          │  │
│  │                   │    │                        │  │
│  │  Settings 窗口     │◄──►│  Pipeline 状态机       │  │
│  │  (Sidebar 布局)   │IPC │  (录音→STT→LLM→粘贴)  │  │
│  │                   │    │                        │  │
│  │  Overlay 窗口     │◄───│  Events (单向推送)      │  │
│  │  (语音输入浮层)    │    │                        │  │
│  └──────────────────┘    └───────────────────────┘  │
│                                                      │
│           ▲ 全局快捷键 (Option+Space / Escape)         │
└─────────────────────────────────────────────────────┘
```

## 进程通信

| 方向 | 机制 | 用途 |
|------|------|------|
| 前端 → 后端 | Tauri IPC Commands | 用户操作触发的命令（配置读写、手动控制录音等） |
| 后端 → 前端 | Tauri Events (`app_handle.emit`) | Pipeline 状态推送，驱动 UI 更新 |

核心事件：`pipeline-state` — 携带 `PipelineEvent { state: PipelineState }` 枚举。

## 后端模块分层

```
lib.rs (入口)
├── 快捷键注册 (tauri_plugin_global_shortcut)
├── STT 模型加载 (启动时自动 / 手动切换)
├── Tray icon + 菜单
│
├── pipeline.rs ──────── 核心状态机
│   状态: Idle → Recording → Transcribing → Optimizing → Pasting → Done
│   支持: Cancelled (ESC) / Error (任意阶段)
│
├── audio/ ────────────── 音频采集 + 转换
│   capture.rs          AudioRecorder (cpal): 系统麦克风 → 原始 PCM
│   converter.rs        stereo→mono, resample→16kHz, i16→f32
│
├── stt/ ──────────────── 语音转录 (多后端)
│   mod.rs              TranscriberBackend trait + ManagedTranscriber
│   whisper_backend.rs  whisper-rs (Metal GPU 加速)
│   sensevoice_backend.rs  sherpa-onnx
│
├── models/ ───────────── 模型管理
│   registry.rs         静态注册表: 模型元数据 + 下载URL + 语言推荐
│   manager.rs          下载/存储/路径/校验
│
├── llm/ ──────────────── 文本优化
│   client.rs           GPT API 调用，流式响应
│
├── input/ ────────────── 系统交互
│   paste.rs            arboard 剪贴板 + 模拟 Cmd+V 粘贴
│   hotkey.rs           快捷键解析 + 转换
│
├── config/ ───────────── 配置
│   mod.rs              TOML 文件读写 (~/.../com.input0.dev/config.toml)
│
├── errors.rs ─────────── 统一错误类型
│   AppError: Config | Audio | Whisper | Llm | Input | Io
│
└── commands/ ─────────── Tauri IPC 命令层 (薄封装)
    audio.rs   config.rs   whisper.rs   llm.rs
    models.rs  input.rs    window.rs
```

**分层原则**：`commands/` 是薄封装层，只做参数解包 + 调用核心模块 + 返回结果。业务逻辑不写在 commands 里。

## 前端模块结构

```
App.tsx (BrowserRouter)
├── / → Settings.tsx ────── 主窗口
│   └── Sidebar.tsx         侧边栏导航
│       ├── HomePage.tsx    首页概览
│       ├── HistoryPage.tsx 转录历史记录
│       └── SettingsPage.tsx 用户配置表单
│
└── /overlay → Overlay.tsx ── 语音输入浮层
    ├── WaveformAnimation.tsx 录音波形动画
    └── ProcessingIndicator.tsx 处理中指示器

stores/ (Zustand, 一个 store per domain)
├── recording-store.ts    录音/Pipeline 状态
├── settings-store.ts     用户配置（与后端 config 同步）
├── history-store.ts      转录历史记录
└── theme-store.ts        暗黑/亮色主题

hooks/
└── useTauriEvents.ts     监听后端 Events，驱动 store 更新
```

**双窗口架构**：Settings 和 Overlay 是两个独立的 Tauri WebView 窗口，通过 URL 路由区分 (`/` vs `/overlay`)。Overlay 窗口透明 + 无边框 + always-on-top。

## 核心数据流

### 语音输入完整流程

```
用户按住 Option+Space
    │
    ▼
lib.rs: on_shortcut(Pressed)
    ├── show_overlay 窗口
    ├── emit PipelineState::Recording
    └── pipeline.start_recording() ─── 开始录音 (cpal)
    │
用户松开 Option+Space
    │
    ▼
lib.rs: on_shortcut(Released)
    └── pipeline.stop_recording_sync() ─── 停止录音
         │
         ▼
    pipeline::process_audio(recorded_data)
         │
         ├── converter: stereo→mono, resample→16kHz
         │   emit PipelineState::Transcribing
         │
         ├── stt: transcribe(audio_f32, language)
         │   emit PipelineState::Optimizing
         │
         ├── llm: optimize_text(raw_text)
         │   emit PipelineState::Pasting
         │
         ├── input: paste(optimized_text)
         │   emit PipelineState::Done { transcribed_text, text }
         │
         └── 2s 后隐藏 Overlay
```

### 取消流程 (ESC)

```
用户按 Escape
    │
    ▼
lib.rs: ESC handler
    ├── pipeline.cancel() ─── 设置 CancellationToken
    ├── emit PipelineState::Cancelled
    └── hide_overlay
```

## 状态管理

### 后端状态 (Rust)

通过 `Tauri::manage()` 注入，全局共享：

| 状态 | 类型 | 用途 |
|------|------|------|
| Pipeline | `Arc<Mutex<Pipeline>>` | 录音器实例 + CancellationToken |
| SharedTranscriber | `Arc<Mutex<ManagedTranscriber>>` | 当前加载的 STT 模型 |

### 前端状态 (React/Zustand)

每个 domain 独立 store，通过 `useTauriEvents` hook 监听后端事件自动更新。

## STT 多后端架构

```rust
trait TranscriberBackend: Send + Sync {
    fn transcribe(&self, audio: &[f32], language: &str) -> Result<String, AppError>;
    fn backend_kind(&self) -> BackendKind;
    fn model_id(&self) -> &str;
}
```

新增 STT 引擎只需：
1. 实现 `TranscriberBackend` trait
2. 在 `models/registry.rs` 注册模型元数据
3. 在 `lib.rs` 的 `load_stt_model` 添加 match 分支

当前后端：
- **WhisperBackend** — whisper-rs，Metal GPU 加速，适合英文和多语言
- **SenseVoiceBackend** — sherpa-onnx，中文/日文/韩文效果更好
