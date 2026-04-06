# Feature: Tauri v2 + React + TypeScript 项目脚手架

## 实现状态

| 步骤 | 状态 |
|------|------|
| 前端初始化 (pnpm + Vite + React 19 + TypeScript) | ✅ 已完成 |
| src-tauri 目录结构和 tauri.conf.json | ✅ 已完成 |
| macOS Info.plist 权限配置 | ✅ 已完成 |
| Cargo.toml 依赖配置 | ✅ 已完成 |
| 前端依赖安装 (zustand, react-router-dom, Tailwind CSS v4, Tauri APIs) | ✅ 已完成 |
| Rust 模块骨架 | ✅ 已完成 |
| 前端页面骨架 | ✅ 已完成 |
| build.rs, resources, capabilities | ✅ 已完成 |
| 编译验证 (pnpm build + cargo build) | ✅ 已通过 |

## 技术决策

### whisper-rs 暂时注释
`whisper-rs` 在 `Cargo.toml` 中被注释掉（`# whisper-rs = ...`），原因是编译 metal 特性需要 libclang 依赖，初始脚手架阶段为确保能顺利编译而先行注释。后续实现语音识别功能时再启用。

### global-shortcut 插件 API
`tauri-plugin-global-shortcut` v2.3.1 使用 `Builder::new().build()` 初始化，而非早期版本的 `init()` 函数。

### 图标文件
`src-tauri/icons/` 目录下创建了最小 RGBA PNG 占位图标（透明黑色），Tauri 构建系统要求图标必须为 RGBA 格式。

## 项目结构

```
input0/
├── src/                    # 前端 React + TypeScript
│   ├── main.tsx            # 入口
│   ├── App.tsx             # 路由
│   ├── index.css           # Tailwind CSS v4 导入
│   ├── vite-env.d.ts       # CSS 模块类型声明
│   ├── pages/
│   │   ├── Settings.tsx    # 设置页面骨架
│   │   └── Overlay.tsx     # 语音输入浮层骨架
│   ├── stores/
│   │   ├── settings-store.ts   # 设置状态 (zustand)
│   │   └── recording-store.ts  # 录音状态 (zustand)
│   └── hooks/
│       └── useTauriEvents.ts   # Tauri 事件监听 (占位)
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # 入口
│   │   ├── lib.rs          # Tauri 应用初始化
│   │   ├── errors.rs       # 统一错误类型
│   │   ├── pipeline.rs     # 语音输入流水线 (占位)
│   │   ├── audio/          # 音频捕获和转换
│   │   ├── whisper/        # Whisper 语音识别
│   │   ├── llm/            # LLM 文本优化
│   │   ├── input/          # 热键和粘贴
│   │   ├── commands/       # Tauri IPC 命令
│   │   └── config/         # 应用配置
│   ├── capabilities/
│   │   └── default.json    # Tauri 权限配置
│   ├── icons/              # 应用图标
│   ├── resources/          # 运行时资源
│   ├── build.rs            # Tauri 构建脚本
│   ├── Cargo.toml          # Rust 依赖
│   ├── tauri.conf.json     # Tauri 配置
│   └── Info.plist          # macOS 权限声明
├── vite.config.ts          # Vite + Tailwind CSS v4 配置
├── tsconfig.json           # TypeScript 配置
├── tsconfig.node.json      # Node TypeScript 配置
├── index.html              # HTML 入口
└── package.json            # 前端依赖和脚本
```

## 关键依赖版本

| 依赖 | 版本 |
|------|------|
| React | 19.x |
| Tauri | 2.10.x |
| Vite | 8.x |
| TypeScript | 6.x |
| Tailwind CSS | 4.x |
| zustand | 5.x |
| react-router-dom | 7.x |
