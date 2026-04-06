# Input0

Input0 是一款专为 macOS 设计的语音输入工具，功能类似于 Typeless。用户只需按住 Option+Space 快捷键即可开始录音，松开按键后，系统会调用本地 Whisper 模型进行语音转文字，并利用 GPT 大模型对文本进行智能优化，最后自动填入当前活跃的输入框。

<!-- [Screenshot Placeholder: Main App Interface] -->

## 功能特性

- 快捷键操作：按住 Option+Space 录音，松开即刻转写。
- 本地转写：集成 whisper-rs 并在 Metal GPU 上加速运行，确保隐私与性能。
- AI 文本优化：通过 GPT 接口自动修正语病、优化措辞。
- 自动上屏：转写优化后的文本自动粘贴至系统当前聚焦的输入框。
- 多语言支持：支持自动识别或手动指定中文、英文、日文、韩文等多种语言。
- 灵活配置：可自定义 API Key、Base URL 以及模型参数。

## 技术栈

- 前端：Tauri v2, React 19, TypeScript, Vite, Tailwind CSS v4, Zustand
- 后端：Rust, whisper-rs (Metal 加速), cpal (音频采集), rubato (重采样), reqwest (LLM 请求)

## 系统要求

- macOS 11.0+
- Apple Silicon 处理器 (推荐以获得最佳 GPU 加速效果)
- 已安装 cmake (`brew install cmake`)
- Rust 稳定版
- Node.js 20+ 及 pnpm

## 开发环境搭建

1. 克隆项目仓库：
   ```bash
   git clone <repository-url>
   cd input0
   ```

2. 安装依赖：
   ```bash
   pnpm install
   ```

3. 下载 Whisper 模型文件：
   项目需要 `ggml-base.bin` 模型文件（约 142MB），请将其放置在 `src-tauri/resources/` 目录下。
   ```bash
   mkdir -p src-tauri/resources
   curl -L -o src-tauri/resources/ggml-base.bin https://huggingface.co/ggerganov/whisper.cpp/resolve/main/models/ggml-base.bin
   ```

## 构建与运行

### 开发模式
启动 Tauri 开发服务器，支持热更新：
```bash
pnpm tauri dev
```

### 生产构建
构建 macOS 应用程序包：
```bash
MACOSX_DEPLOYMENT_TARGET=11.0 CMAKE_OSX_DEPLOYMENT_TARGET=11.0 pnpm tauri build --bundles app
```

### 运行测试
执行后端逻辑测试：
```bash
cd src-tauri && cargo test --lib
```

## 项目结构

```text
input0/
├── src/                    # React 前端
│   ├── pages/              # 设置页面、悬浮窗
│   ├── stores/             # Zustand 状态管理
│   ├── hooks/              # Tauri 事件钩子
│   └── components/         # UI 组件
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── audio/          # 音频录制与转换逻辑
│   │   ├── whisper/        # 本地 STT 转写模块
│   │   ├── llm/            # GPT 文本优化接口
│   │   ├── input/          # 剪贴板操作与模拟粘贴
│   │   ├── config/         # TOML 配置文件处理
│   │   ├── commands/       # Tauri IPC 指令
│   │   ├── pipeline.rs     # 语音处理流水线状态机
│   │   └── lib.rs          # 应用入口与生命周期管理
│   └── resources/          # Whisper 模型文件 (*.bin)
└── docs/                   # 设计文档与需求说明
```

## 配置说明

应用配置文件位于：
`~/Library/Application Support/com.input0.dev/config.toml`

主要配置项包括：
- `api_key`: LLM 服务的 API 密钥。
- `base_url`: LLM 服务地址。
- `language`: 转写语言设置 (auto/zh/en/ja/ko/fr/de/es/ru)。
- `hotkey`: 唤起快捷键。

## License

本项目采用 [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/) 许可证。你可以自由分享和修改本项目，但不得用于商业用途。
