# Input0

macOS 语音输入工具 — 按住快捷键录音，松开后本地 STT 转文字 → LLM 优化 → 自动粘贴到当前输入框。

## Quick Start

```bash
# 安装前端依赖
pnpm install

# 开发模式（Tauri + Vite 热更新）
pnpm tauri dev

# 运行 Rust 后端测试
cd src-tauri && cargo test --lib

# 前端类型检查
pnpm build   # tsc && vite build

# 生产构建
MACOSX_DEPLOYMENT_TARGET=11.0 CMAKE_OSX_DEPLOYMENT_TARGET=11.0 pnpm tauri build --bundles app
```

前置要求：macOS 11+、Rust stable、Node.js 20+、pnpm、cmake (`brew install cmake`)。首次运行需下载 STT 模型（在 Settings 页面触发）。

## Architecture Overview

Tauri v2 桌面应用，双进程架构：

- **Rust 后端** (`src-tauri/src/`): 音频采集 → 格式转换 → STT 转录 → LLM 优化 → 剪贴板粘贴，通过 Pipeline 状态机驱动。
- **React 前端** (`src/`): 两个窗口 — Settings（主窗口，含 Sidebar 多页面）+ Overlay（语音输入浮层）。
- **通信**: Tauri IPC commands (前端→后端) + Events (后端→前端，`pipeline-state` 事件驱动 UI 状态)。

详见 → [docs/architecture.md](docs/architecture.md)

## Directory Structure

```
.github/workflows/            # CI/CD
└── release.yml               # 推送 release 分支 → 双架构构建 + GitHub Release

src-tauri/src/                # Rust 后端
├── pipeline.rs               # 核心：语音处理流水线状态机
├── lib.rs                    # 应用入口、快捷键注册、STT 模型加载
├── errors.rs                 # 统一错误类型 AppError (thiserror)
├── audio/                    # 音频采集(cpal) + 格式转换(rubato)
│   ├── capture.rs            # AudioRecorder - 麦克风录音
│   ├── converter.rs          # stereo→mono, 重采样16kHz, i16→f32
│   └── tests.rs              # 音频转换单元测试
├── stt/                      # STT 转录抽象层
│   ├── mod.rs                # TranscriberBackend trait + ManagedTranscriber
│   ├── whisper_backend.rs    # whisper-rs 后端 (Metal GPU 加速)
│   ├── sensevoice_backend.rs # sherpa-onnx SenseVoice 后端
│   ├── paraformer_backend.rs # sherpa-onnx Paraformer 后端
│   └── moonshine_backend.rs  # sherpa-onnx Moonshine 后端
├── models/                   # STT 模型管理
│   ├── registry.rs           # 模型注册表（元数据、下载URL、语言推荐）
│   └── manager.rs            # 模型下载/存储/校验
├── llm/                      # LLM 文本优化（GPT API）
├── input/                    # 剪贴板操作 + 模拟粘贴(arboard)
├── config/                   # TOML 配置文件读写
├── vocabulary.rs             # 词汇库存储（JSON 持久化、去重、原子写入）
└── commands/                 # Tauri IPC 命令（每个模块一个文件）
    ├── audio.rs              # start/stop/toggle/cancel_pipeline
    ├── config.rs             # get/save/update_config_field
    ├── whisper.rs            # transcribe/init/is_loaded
    ├── llm.rs                # optimize_text/test_api_connection
    ├── models.rs             # list/download/switch/delete/recommend
    ├── vocabulary.rs         # get/add/remove/validate_and_add vocabulary
    ├── input.rs              # paste_text/parse_hotkey
    └── window.rs             # show/hide overlay/settings

src/                          # React 前端
├── App.tsx                   # BrowserRouter: / → Settings, /overlay → Overlay
├── pages/
│   ├── Settings.tsx          # 主窗口（Sidebar 布局）
│   └── Overlay.tsx           # 语音输入浮层（液态玻璃 UI）
├── components/
│   ├── Sidebar.tsx           # 侧边栏导航
│   ├── HomePage.tsx          # 首页
│   ├── HistoryPage.tsx       # 转录历史记录
│   ├── SettingsPage.tsx      # 配置页面
│   ├── WaveformAnimation.tsx # 录音波形动画
│   ├── ProcessingIndicator.tsx # 处理中指示器
│   └── Toast.tsx             # 提示消息
├── stores/                   # Zustand 状态管理
│   ├── recording-store.ts    # 录音状态
│   ├── settings-store.ts     # 用户配置
│   ├── history-store.ts      # 转录历史
│   ├── theme-store.ts        # 主题状态
│   └── update-store.ts       # 应用更新状态
└── hooks/
    └── useTauriEvents.ts     # Tauri 事件监听 Hook

docs/                         # 设计文档与需求记录
```

## Key Conventions

1. **禁止格式化老代码**: 修改文件时只改逻辑代码，不调整老代码的缩进、空格、空行。Git diff 只包含实质变更。
2. **Feature 文档驱动**: 每个新需求先建 `docs/feature-xxx.md`，记录需求分析、技术方案、实现状态，完成后更新本文件索引。
3. **TDD (后端)**: Rust API 实现先写测试再写代码。测试在模块内的 `tests.rs` 或 `#[cfg(test)]` 中。
4. **错误处理**: 统一使用 `AppError` (thiserror)，按模块分类: Config / Audio / Whisper / Llm / Input / Io。禁止 `unwrap()` / `expect()` 在非初始化代码中使用。
5. **前后端通信**: 后端→前端用 Events (`app_handle.emit("pipeline-state", ...)`)，前端→后端用 Tauri IPC commands。
6. **状态管理**: 前端用 Zustand (一个 store per domain)，后端用 `Arc<Mutex<T>>` 通过 Tauri managed state 共享。
7. **STT 架构**: 通过 `TranscriberBackend` trait 抽象多后端，新增 STT 引擎只需实现该 trait。
8. **类型安全**: TypeScript `strict: true`，Rust 默认 warnings。禁止 `as any` / `@ts-ignore` / `@ts-expect-error`。
9. **Commit message 必须英文**: 所有 git commit message（包括 subject 和 body）使用英文书写，遵循 Conventional Commits 风格（`feat:` / `fix:` / `docs:` / `chore:` 等）。`docs` 和 `chore` 类型不会出现在 GitHub Release 页面（由 `cliff.toml` 过滤）。

详见 → [docs/conventions.md](docs/conventions.md)

## Documentation Map

| 文档 | 用途 | 最后校验 |
|------|------|----------|
| [docs/architecture.md](docs/architecture.md) | 系统架构、模块关系、数据流、分层设计 | 2026-04-03 |
| [docs/conventions.md](docs/conventions.md) | 编码规范、命名约定、模式参考 | 2026-04-03 |
| [docs/init-prd.md](docs/init-prd.md) | 项目初始需求文档（PRD） | 2026-03-29 |
| [docs/feature-scaffold.md](docs/feature-scaffold.md) | Tauri v2 + React + TS 脚手架搭建记录 | 2026-03-29 |
| [docs/feature-audio-converter.md](docs/feature-audio-converter.md) | 音频转换模块（TDD 实现） | 2026-03-29 |
| [docs/feature-pipeline.md](docs/feature-pipeline.md) | 语音处理流水线 + 启动流程 | 2026-03-29 |
| [docs/feature-zh-initial-prompt.md](docs/feature-zh-initial-prompt.md) | 中文转录简繁体修复 | 2026-03-29 |
| [docs/feature-model-switching.md](docs/feature-model-switching.md) | STT 模型切换 + 按需下载 + 推荐 | 2026-04-19 |
| [docs/research-local-stt-models.md](docs/research-local-stt-models.md) | 本地 STT 模型调研报告 | 2026-04-19 |
| [docs/feature-add-stt-models.md](docs/feature-add-stt-models.md) | 新增三款 STT 模型：FireRedASR v1 / Paraformer 三语 / Zipformer 中文 CTC | 2026-04-19 |
| [docs/feature-ui-redesign.md](docs/feature-ui-redesign.md) | UI 重设计：暗黑主题 + 动画 | 2026-03-30 |
| [docs/feature-esc-cancel.md](docs/feature-esc-cancel.md) | ESC 取消语音输入 | 2026-03-31 |
| [docs/feature-ui-redesign-sidebar.md](docs/feature-ui-redesign-sidebar.md) | Sidebar + 多页面布局 | 2026-04-01 |
| [docs/feature-prompt-optimization.md](docs/feature-prompt-optimization.md) | LLM 文本纠错 Prompt 优化 + 历史上下文 | 2026-04-03 |
| [docs/landing-page-brief.md](docs/landing-page-brief.md) | Landing Page Brief：品牌文案、功能清单、设计系统规范 | 2026-04-19 |
| [docs/feature-text-structuring.md](docs/feature-text-structuring.md) | 文本结构化优化（可选 toggle，LLM 自动换行/列表/段落） | 2026-04-10 |
| [docs/feature-vocabulary.md](docs/feature-vocabulary.md) | 词汇库：手动管理 + 自动学习 + Prompt 注入（纯正确词汇模型） | 2026-04-06 |
| [docs/feature-active-app-context.md](docs/feature-active-app-context.md) | 活跃应用名称作为 LLM 轻量级领域信号 | 2026-04-06 |
| [docs/feature-user-tags.md](docs/feature-user-tags.md) | 用户偏好标签：职业/领域/工作空间标签注入 LLM system prompt | 2026-04-06 |
| [docs/feature-auto-update.md](docs/feature-auto-update.md) | 应用内自动更新：tauri-plugin-updater + UI 通知 | 2026-04-06 |
| [docs/feature-single-key-hotkey.md](docs/feature-single-key-hotkey.md) | 单键长按触发录音：CGEventTap 支持左右分离修饰键 | 2026-04-25 |

> 新需求 → 新建 `docs/feature-xxx.md` → 更新此表（含最后校验日期）。

## Common Tasks

| 任务 | 入手点 |
|------|--------|
| 添加新 Tauri IPC 命令 | `src-tauri/src/commands/` 新建或扩展模块 → 在 `lib.rs` 的 `invoke_handler` 注册 |
| 添加新 STT 后端 | 实现 `stt::TranscriberBackend` trait → 在 `models/registry.rs` 注册 → `lib.rs` 的 `load_stt_model` 添加分支 |
| 修改 Pipeline 流程 | `src-tauri/src/pipeline.rs` — PipelineState 枚举 + process_audio 函数 |
| 添加前端页面 | `src/components/` 新建组件 → `src/pages/Settings.tsx` 中添加路由 → `Sidebar.tsx` 添加导航项 |
| 修改配置项 | `src-tauri/src/config/` 添加字段 → `src/stores/settings-store.ts` 同步 → `SettingsPage.tsx` 添加 UI |
| 修改快捷键行为 | `src-tauri/src/lib.rs` — `on_shortcut` 回调 |
| 调试 Overlay UI | `src/pages/Overlay.tsx` + `src/components/WaveformAnimation.tsx` |
| 模型管理 | `src-tauri/src/models/` — registry.rs (元数据) + manager.rs (下载/路径) |

## Development Workflow

1. **需求分析** → 在 `docs/` 下创建 `feature-xxx.md`，记录需求、技术方案、实现计划
2. **实现** → TDD（后端先测试）→ 在 feature 文档中更新状态（待开始 / 进行中 / 已完成 / 已验证）
3. **收尾** → 更新本文件 Documentation Map 表格 → 运行 `cargo test --lib` + `pnpm build` 验证

## Documentation Maintenance (Agent 自动执行)

文档索引表中的「最后校验」列记录了每篇文档上次被确认与代码一致的日期。Agent 通过以下两条规则保持文档鲜活：

### 规则 1：任务触发同步（实时）

当 Agent 执行任务时，如果改动可能导致某篇文档内容过时，**必须在同一任务中同步更新该文档**。

触发条件（任一满足即触发）：
- 修改了文档中描述的模块接口、函数签名、数据流
- 新增/删除/重命名了文件或目录（可能影响 Directory Structure 或 architecture.md）
- 修改了配置项结构（影响 conventions.md 或相关 feature 文档）
- 修改了 Pipeline 状态机、前后端通信协议等核心流程
- **新增/修改/删除了用户可见功能**（影响 `docs/landing-page-brief.md` 的功能清单、Slogan 或设计规范）

执行方式：
1. 完成代码改动后，检查 Documentation Map 中哪些文档可能受影响
2. 阅读受影响的文档，对比当前代码，修正过时内容
3. 更新文档索引表中对应文档的「最后校验」日期为当天
4. 如果改动涉及 AGENTS.md 本身的 Directory Structure 或 Architecture Overview，也需同步更新
5. **Landing Page 同步**：如果改动涉及功能增删改（新功能、功能变更、功能移除）、设计系统变更（色彩/字体/圆角/动效参数）、或品牌信息变更，必须同步更新 `docs/landing-page-brief.md` 中对应的章节（功能清单、设计系统规范、技术栈等）

### 规则 2：定时校验（任务结束时）

每次用户与 Agent 的对话即将结束、当前任务已完成时，Agent 检查 Documentation Map 索引表：

1. 找出「最后校验」日期 **早于 7 天前** 的文档
2. 如果存在过期文档，启动子任务（后台执行，不阻塞用户）：
   - 逐一阅读过期文档，与当前代码对比
   - 修正已过时的内容（接口变更、文件增删、流程变化等）
   - 更新「最后校验」日期为当天
   - 如果文档内容完全准确无需修改，也更新日期（表示已校验）
3. 如果没有过期文档，不做任何操作

### 注意事项

- **不要打扰用户**：定时校验在后台静默执行，不需要用户确认
- **只改过时内容**：遵守「禁止格式化老代码」规则，只修正与代码脱节的描述
- **保留文档风格**：每篇文档有自己的写作风格和详略程度，更新时保持一致
- **优先级**：architecture.md 和 conventions.md 是核心文档，优先校验
