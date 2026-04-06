# Coding Conventions

本文档记录 Input0 项目的编码规范和约定。所有 agent 和人类贡献者都应遵循。

## 通用规则

### Git Diff 纪律

**最重要的规则：** 修改文件时只改动与当前任务相关的逻辑代码。

- ❌ 不调整老代码的缩进、空格、空行
- ❌ 不删除行尾空格
- ❌ 不修改空行数量
- ❌ 不重新格式化未改动的代码块
- ✅ Git diff 中只包含实质性的逻辑变更

**为什么**：保持 git blame 可追溯，避免无关噪音干扰 code review。

### Feature 文档驱动开发

每个新需求按以下流程：

1. 在 `docs/` 下创建 `feature-xxx.md`
2. 记录：需求分析、技术方案、实现计划
3. 实现过程中更新状态（待开始 / 进行中 / 已完成 / 已验证）
4. 完成后在 `AGENTS.md` 的 Documentation Map 表格中添加条目

文档模板参考 `docs/feature-model-switching.md`。

## Rust 后端

### 错误处理

统一使用 `AppError`（基于 thiserror），按模块分类：

```rust
pub enum AppError {
    Config(String),
    Audio(String),
    Whisper(String),
    Llm(String),
    Input(String),
    Io(#[from] std::io::Error),
}
```

**规则**：
- 所有可失败函数返回 `Result<T, AppError>`
- 禁止在非初始化代码中使用 `unwrap()` / `expect()`
  - 初始化代码（如 `lib.rs` 的 `setup`）中允许有限使用
  - 测试代码中允许使用
- 新模块如果现有错误变体不够用，应扩展 `AppError` 枚举

### TDD

Rust API 实现采用测试驱动开发：

- 先写测试，再写实现
- 测试位置：模块内的 `tests.rs` 文件（如 `audio/tests.rs`）或 `#[cfg(test)]` 块
- 运行测试：`cd src-tauri && cargo test --lib`

### Tauri Commands

`commands/` 模块是薄封装层：

```rust
// ✅ 正确：commands 只做参数解包 + 调用核心模块
#[tauri::command]
async fn start_recording(pipeline: State<'_, Arc<Mutex<Pipeline>>>, ...) -> Result<(), AppError> {
    let mut p = pipeline.lock().map_err(...)?;
    p.start_recording(&app_handle)
}

// ❌ 错误：业务逻辑写在 commands 里
#[tauri::command]
async fn start_recording(...) -> Result<(), AppError> {
    // 100 行录音逻辑直接写在这里
}
```

添加新命令的步骤：
1. 在 `commands/` 中新建或扩展模块文件
2. 在 `commands/mod.rs` 中 `pub mod` 导出
3. 在 `lib.rs` 的 `invoke_handler` 宏中注册

### 并发模式

后端共享状态通过 `Arc<Mutex<T>>` + Tauri `manage()` 注入：

```rust
// 注册
.manage(pipeline::new_managed())       // Arc<Mutex<Pipeline>>
.manage(stt::new_shared_transcriber()) // Arc<Mutex<ManagedTranscriber>>

// 使用（在 commands 中）
fn foo(pipeline: State<'_, Arc<Mutex<Pipeline>>>) { ... }
```

CoreAudio 等可能阻塞的调用使用 `tokio::task::spawn_blocking` 避免阻塞异步运行时。

### 命名约定

- 模块：snake_case（`whisper_backend.rs`）
- 类型：PascalCase（`AudioRecorder`, `PipelineState`）
- 函数：snake_case（`start_recording`, `process_audio`）
- Tauri 命令：snake_case（前端调用时也用 snake_case）

## React 前端

### 状态管理

使用 Zustand，一个 store per domain：

```typescript
// ✅ 正确：每个 domain 独立 store
// stores/recording-store.ts  — 录音/Pipeline 状态
// stores/settings-store.ts   — 用户配置
// stores/history-store.ts    — 转录历史
// stores/theme-store.ts      — 主题

// ❌ 错误：一个巨大的全局 store
```

### 前后端通信

```typescript
// 前端 → 后端：Tauri IPC commands
import { invoke } from '@tauri-apps/api/core';
await invoke('get_config');

// 后端 → 前端：Events (通过 useTauriEvents hook 监听)
import { listen } from '@tauri-apps/api/event';
listen('pipeline-state', (event) => { ... });
```

### 组件结构

- 页面级组件放 `src/pages/`（Settings, Overlay）
- 功能组件放 `src/components/`（与路由无关的独立组件）
- 新页面添加流程：`src/components/` 新建组件 → `Settings.tsx` 中注册 → `Sidebar.tsx` 添加导航项

### 类型安全

- TypeScript `strict: true` 已启用
- 禁止 `as any` / `@ts-ignore` / `@ts-expect-error`
- 类型检查命令：`pnpm build`（会先运行 `tsc`）

### 样式

- Tailwind CSS v4（通过 Vite 插件集成）
- 暗黑主题优先（Overlay 使用液态玻璃效果）
- 动画使用 Framer Motion

### 命名约定

- 组件文件：PascalCase（`HomePage.tsx`, `WaveformAnimation.tsx`）
- Store 文件：kebab-case（`recording-store.ts`）
- Hook 文件：camelCase with `use` 前缀（`useTauriEvents.ts`）
