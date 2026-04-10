<div align="right">
  <a href="README.md">English</a> | <strong>简体中文</strong>
</div>

<div align="center">
  <img src="landing-page/logo.png" alt="Input 0 Logo" width="120" />
  <h1>Input 0</h1>
</div>

> 🏗️ 本项目使用 [Harness Engineering](https://github.com/10xChengTu/harness-engineering) 进行 AI Agent Harness 搭建。

macOS 语音输入工具 — 按住快捷键录音，松开后自动转写、优化、粘贴到当前输入框。

本地 AI 转录 → LLM 文本优化 → 自动粘贴。隐私、快速、零摩擦。

![Input 0 主界面](images/screenshot.png)

## 功能特性

- **一键语音输入** — 按住 `Option+Space`（可自定义）录音，松开即自动转写 + 优化 + 粘贴，无需切换窗口。
- **隐私优先的本地转录** — 四大 AI 引擎（Whisper、SenseVoice、Paraformer、Moonshine）通过 Metal GPU 完全在你的 Mac 上本地运行，音频数据永不离开设备。
- **AI 文本优化** — LLM 自动修正语病、去除口吃和语气词、结构化表达。内置 40+ 技术术语拼音纠错（如"瑞嗯特" → "React"），支持自定义词汇库。
- **自动粘贴** — 优化后的文本自动粘贴到当前聚焦的输入框 — Slack、微信、VS Code、浏览器，任何 App。
- **99+ 语言支持** — 4 大引擎、9 个模型，覆盖 99+ 种语言，系统根据你的语言自动推荐最佳模型。
- **模型按需管理** — 应用体积轻量，STT 模型按需下载。一键切换、进度显示、智能推荐。
- **ESC 取消** — 录音、转写、优化过程中随时按 ESC 取消，不会有任何文字被粘贴。
- **历史记录** — 查看最近的语音转录原文与 AI 优化结果，支持对比和一键复制。
- **自定义词汇库** — 手动添加专业术语、人名、产品名，AI 转录优化时优先使用。支持自动学习，LLM 验证后自动加入词汇库。
- **暗黑 / 亮色主题** — 双主题支持，匹配你的使用偏好。
## 系统要求

- macOS 11.0+
- 推荐 Apple Silicon 处理器（以获得 Metal GPU 加速效果）

## 新手上路

### 1. 安装

从 [GitHub Releases](https://github.com/nicepkg/input0/releases) 下载最新的 `.dmg` 文件，打开后将 **Input 0** 拖入「应用程序」文件夹即可。

> 首次打开时，macOS 可能会弹出安全提示。前往 **系统设置 → 隐私与安全性**，点击 **仍要打开** 即可。

### 2. 授予权限

Input 0 需要以下 macOS 权限才能正常工作，首次启动时会自动弹出请求：

| 权限 | 用途 |
|---|---|
| **麦克风** | 录制你的语音 |
| **辅助功能** | 模拟 Cmd+V 将文本粘贴到其他应用 |

前往 **系统设置 → 隐私与安全性** 授权后，重启应用即可生效。

### 3. 配置 LLM API Key ⚡

> **此步骤为必需。** 如果不配置 API Key，语音转文字仍可正常使用，但 AI 文本优化功能（语法修正、去口头禅、结构化）将被跳过。

1. 打开 Input 0 → 点击侧边栏的 **⚙️ 设置** 图标
2. 找到 **LLM API** 区域
3. 填写以下信息：

| 字段 | 说明 | 默认值 |
|---|---|---|
| **API Key** | 你的 OpenAI（或兼容服务）API 密钥 | *（空，必须填写）* |
| **API Base URL** | 服务端点地址 | `https://api.openai.com/v1` |
| **Model** | LLM 模型名称 | `gpt-4o-mini` |

**使用第三方服务？**（如 Azure OpenAI、Groq、本地 Ollama 或任何 OpenAI 兼容 API）
- 将 **API Base URL** 改为你的服务商端点
- 将 **Model** 改为对应的模型名称
- 填入对应的 **API Key**

点击 **测试连接** 按钮验证 API Key 是否可用。

### 4. 下载 STT 模型 ⚡

> **此步骤为必需。** 你需要至少下载一个模型才能将语音转为文字。

1. 打开 Input 0 → 进入 **设置** → **模型** 区域
2. 应用会根据你的语言设置推荐最佳模型 — 跟随推荐或自行选择
3. 点击模型旁边的 **下载** 按钮
4. 等待下载完成（有进度条显示）

**选哪个模型好？**

| 你说的语言 | 推荐模型 | 大小 |
|---|---|---|
| 中文 | SenseVoice Small | ~228 MB |
| 英语 | Whisper Large v3 Turbo | ~1.5 GB |
| 日语 / 韩语 | SenseVoice Small | ~228 MB |
| 多语言混用 | Whisper Large v3 | ~2.9 GB |
| 追求最快速度 | Moonshine Base（仅英文） | ~274 MB |
| 追求最小体积 | Whisper Base | ~142 MB |

### 5. 开始使用

1. 按住 **Option+Space**（或你自定义的快捷键）
2. 自然说话
3. 松开按键 — 文字自动转写、优化、粘贴到当前输入框

就这么简单！🎉

## 常见问题

### 模型下载失败

如果模型下载失败或卡住：

1. **检查网络** — 模型从 Hugging Face 下载。如果你所在的网络环境无法顺畅访问 Hugging Face，建议使用 VPN 或代理。
2. **重试** — 关闭并重新打开应用，再次尝试下载。已下载的部分会自动续传，无需从头开始。
3. **磁盘空间** — 确保有足够的剩余空间。最大的模型（Whisper Large v3）需要约 2.9 GB。
4. **手动下载** — 作为最后手段，可以手动下载模型文件并放入：
   ```
   ~/Library/Application Support/com.input0/models/<模型ID>/
   ```
   具体文件名和下载地址请参考 [模型注册表](src-tauri/src/models/registry.rs)。

### 说完话没有文字输出

- 确认已下载并选中了至少一个 STT 模型
- 检查 **麦克风** 权限是否已授予
- 检查 **辅助功能** 权限是否已授予（自动粘贴需要此权限）

### AI 优化不生效

- 确认已在设置中正确填写 **API Key**
- 点击 **测试连接** 检查 API 是否可达
- 如使用第三方服务，仔细核对 **API Base URL** 和 **Model** 名称

### 快捷键无响应

- 确认没有其他应用占用相同快捷键
- 尝试在 **设置** 中更换快捷键
- 更换后重启应用

## 全部 STT 模型

| 模型 | 大小 | 最佳场景 |
|------|------|---------|
| Whisper Base | ~142 MB | 快速轻量，适合日常使用 |
| Whisper Small | ~466 MB | 性价比高，精度与速度均衡 |
| Whisper Medium | ~1.4 GB | 多语言转录精度优秀 |
| Whisper Large v3 | ~2.9 GB | 最高精度，99 种语言 |
| Whisper Large v3 Turbo | ~1.5 GB | 英文/多语言最高精度 |
| Whisper Large v3 Turbo Q5 | ~547 MB | 高精度量化版，平衡大小与质量 |
| SenseVoice Small | ~228 MB | 中文/日文/韩文识别最佳 |
| Paraformer 中文 | ~217 MB | 中文专用，推理极快 |
| Moonshine Base (EN) | ~274 MB | 英文专用，速度约为 Whisper 的 5 倍 |

## 许可证

本项目采用 [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/) 许可证。你可以自由分享和修改，但不得用于商业用途。
