# Input 0 — Landing Page Brief

> 本文档汇总了 Input 0 的品牌文案、功能清单、设计系统规范，供后续生成 Landing Page 时作为 Prompt 输入使用。

---

## 一、品牌定位

**产品名称**：Input 0（代码库中写作 Input0，品牌展示建议写作 Input 0）

**一句话定位**：macOS 上的 AI 语音输入工具 — 按住快捷键说话，松开即获得完美文本，自动粘贴到任何输入框。

**品牌名寓意**："Input 0" = 零摩擦输入。不需要打字，不需要切换窗口，不需要复制粘贴 — 从想法到文字，中间步骤为零。

---

## 二、品牌 Slogan 候选

### 主 Slogan（推荐三选一）

| # | Slogan | 调性 | 说明 |
|---|--------|------|------|
| 1 | **Zero typing. Pure voice.** | 简洁克制 | 玩转品牌名中的 "0"，直接传达核心价值 |
| 2 | **Speak. It's done.** | 行动导向 | 极简两词，暗示零摩擦的完整闭环 |
| 3 | **From voice to perfect text. Zero effort.** | 功能叙事 | 完整描述产品价值，适合首屏副标题 |

### 副标题 / 支撑文案

- "Press a key. Speak naturally. Get polished text — pasted automatically."
- "Local AI transcription + intelligent text optimization. Private, fast, effortless."
- "Your thoughts, perfectly written. No typing required."

### 中文版 Slogan

| 原文案 | 新 Slogan | 说明 |
|--------|-----------|------|
| "自然说话，完美书写" | **"开口即书写，落笔即完美"** | 保持对仗，融入"零输入"的即时感 |
| （备选） | **"不打字，只说话"** | 极简否定式，与 "Don't type, just speak" 呼应 |
| （备选） | **"说出来，就写好了"** | 口语化，强调自动完成的惊喜感 |
| （备选） | **"零输入，全语音"** | 直接扣品牌名 |

---

## 三、核心功能清单（Landing Page 展示用）

### 3.1 一键语音输入

**标题**：Press & Speak
**描述**：按住 `Option+Space`（可自定义）开始录音，松开即自动转写 + 优化 + 粘贴。全程无需切换窗口。
**关键词**：一键操作、全局快捷键、无缝体验

### 3.2 本地 AI 转录

**标题**：Privacy-First Architecture
**描述**：Whisper、SenseVoice、Paraformer、Moonshine——四大本地 AI 引擎通过 Metal GPU 完全在你的 Mac 上本地运行，音频数据永不离开设备。文本优化可连接你自己的 LLM API，完全掌控数据流。
**关键词**：四大引擎、本地模型、隐私保护、Metal 加速、Apple Silicon 优化
**支持模型**：

| 模型 | 大小 | 最佳场景 |
|------|------|---------|
| Whisper Base | ~142MB | 快速轻量，适合日常使用 |
| Whisper Small | ~466MB | 性价比高，精度与速度均衡 |
| Whisper Medium | ~1.4GB | 多语言转录精度优秀 |
| Whisper Large v3 | ~2.9GB | 最高精度，99 种语言 |
| Whisper Large v3 Turbo | ~1.5GB | 英文/多语言最高精度 |
| Whisper Large v3 Turbo Q5 | ~547MB | 高精度量化版，平衡大小与质量 |
| SenseVoice Small | ~228MB | 中文/日文/韩文识别最佳 |
| Paraformer 中文 | ~217MB | 中文专用，推理极快 |
| Moonshine Base (EN) | ~274MB | 英文专用，速度约为 Whisper 的 5 倍 |

### 3.3 AI 文本优化

**标题**：AI-Powered Polish
**描述**：GPT 自动修正语病、去除口吃和语气词、结构化表达。内置 40+ 技术术语拼音纠错（如"瑞嗯特" → "React"），支持自定义词汇库优先纠正，开发者友好。
**关键词**：LLM 优化、技术术语修正、语境感知（10 条历史上下文）、自定义词汇库

### 3.4 自动粘贴

**标题**：Auto-Paste Anywhere
**描述**：优化后的文本自动粘贴到你当前聚焦的输入框 — Slack、微信、VS Code、浏览器，任何 App。
**关键词**：系统级粘贴、全应用兼容

### 3.5 多语言支持

**标题**：Speak Any Language
**描述**：四大 AI 引擎、9 个模型，覆盖 99+ 种语言，支持中文、English、日本語、한국어、Español、Français、Deutsch 等。系统会根据你的语言自动推荐最佳模型。
**关键词**：多语言、自动检测、智能推荐、99+ 语言

### 3.6 模型按需管理

**标题**：Download What You Need
**描述**：应用体积轻量，STT 模型按需下载。支持一键切换、进度显示、删除不需要的模型。系统会根据你的语言自动推荐最佳模型。
**关键词**：按需下载、模型管理、智能推荐

### 3.7 随时取消

**标题**：ESC to Cancel
**描述**：录音、转写、优化过程中随时按 ESC 取消。不会有任何文字被粘贴。
**关键词**：可控、可撤销

### 3.8 历史记录

**标题**：Review Your History
**描述**：查看最近的语音转录原文与 AI 优化结果。支持筛选、一键复制、清除。
**关键词**：转录历史、对比原文与优化

### 3.9 暗黑 / 亮色主题

**标题**：Dark & Light
**描述**：支持暗黑和亮色两套主题，跟随你的审美偏好。
**关键词**：双主题、视觉舒适

### 3.10 液态玻璃 Overlay

**标题**：Liquid Glass UI
**描述**：录音时屏幕底部出现的浮层采用 macOS 原生毛玻璃效果，不遮挡工作区，沉浸式体验。
**关键词**：毛玻璃、macOS 原生、沉浸式

### 3.11 自定义词汇库

**标题**：Your Vocabulary, Your Rules
**描述**：手动添加专业术语、人名、产品名等词汇，AI 转录优化时优先使用。系统还能自动学习 — 当你修改转录结果时，自动检测词汇纠正并通过 LLM 验证后加入词汇库。
**关键词**：自定义词汇、自动学习、智能纠正、LLM 验证

---

## 四、设计系统规范

> **统一设计系统**：桌面端应用与 Landing Page 共享同一套设计系统（Material Design 3 色彩体系 + Inter 字体 + Milled Button 样式）。以下规范同时适用于两端。

### 4.1 色彩体系（MD3 Surface Hierarchy）

#### 亮色主题（Light）

| Token | 值 | 用途 |
|-------|---|------|
| surface | `#f9f9f9` | 页面背景 |
| surface-container-lowest | `#ffffff` | 卡片背景 |
| surface-container-low | `#f3f3f3` | 次级容器 |
| surface-container | `#eeeeee` | 容器 |
| surface-container-high | `#e8e8e8` | 高层容器 |
| on-surface | `#1a1c1c` | 主文字 |
| on-surface-variant | `#474747` | 次级文字 |
| outline | `#777777` | 三级文字 / 图标 |
| outline-variant | `#c6c6c6` | 边框 |
| primary | `#171717` | 强调色 |
| primary-container | `#3c3b3b` | Milled Button 渐变起点 |

#### 暗色主题（Dark）

| Token | 值 | 用途 |
|-------|---|------|
| surface | `#121212` | 页面背景 |
| surface-container-lowest | `#0a0a0a` | 卡片背景 |
| surface-container-low | `#1a1a1a` | 次级容器 |
| surface-container | `#1e1e1e` | 容器 |
| surface-container-high | `#252525` | 高层容器 |
| on-surface | `#e6e6e6` | 主文字 |
| on-surface-variant | `#a0a0a0` | 次级文字 |
| outline | `#5e5e5e` | 三级文字 / 图标 |
| outline-variant | `#3b3b3b` | 边框 |
| primary | `#e5e5e5` | 强调色 |
| primary-container | `#3c3b3b` | Milled Button 渐变起点 |

#### 通用色彩

| Token | 值 | 用途 |
|-------|---|------|
| error | `#ba1a1a` | 错误提示 |
| success | `#22c55e` | 成功状态 |
| glass-bg | `rgba(0,0,0,0.55)` | 液态玻璃背景 |
| glass-border | `rgba(255,255,255,0.12)` | 液态玻璃边框 |
| glass-glow | `rgba(255,255,255,0.06)` | 液态玻璃发光 |

### 4.2 字体

```
font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif;
-webkit-font-smoothing: antialiased;
```

主字体为 Inter（Google Fonts CDN），SF Pro 作为回退。Landing Page 标题使用 `tracking-tighter` + `font-extrabold` 强化冲击力。

### 4.3 圆角

| Token | 值 |
|-------|---|
| radius-sm | 4px |
| radius-md | 8px |
| radius-lg | 12px |
| radius-xl | 16px |
| radius-2xl | 20px |

### 4.4 动效规范

| 名称 | 参数 | 用途 |
|------|------|------|
| Quick Spring | `stiffness:400, damping:30, mass:0.5` | 按钮、Toast、Overlay 入场 |
| Natural Spring | `stiffness:260, damping:25, mass:0.8` | 区域入场、页面切换 |
| Ease Spring (CSS) | `cubic-bezier(0.25, 1, 0.5, 1)` | 交互反馈、Feature Card hover |
| Theme Transition | `0.2s ease` | 主题切换时的背景/文字过渡 |

动画库：**Framer Motion v12**（React 19 兼容）

### 4.5 Overlay 视觉效果

```
// Pill 样式
backgroundColor: rgba(0, 0, 0, 0.80)
borderColor: rgba(255, 255, 255, 0.20)
boxShadow: 0 8px 32px rgba(0,0,0,0.4), inset 0 1px 0 rgba(255,255,255,0.08)

// 尺寸
高度: 32px
宽度: 120px（error 时自适应）
圆角: full (pill shape)

// 原生效果
macOS NSPanel + native blur view
```

### 4.6 组件模式

| 模式 | 样式组合 |
|------|---------|
| Card | `surface-container-lowest + rounded-xl + outline-variant border` |
| Milled Button | `.milled-button` 类：`radial-gradient(primary-container → primary)` + hover 渐变 + active scale(0.97) |
| Secondary Button | `btn-secondary-bg + outline-variant border + hover:btn-secondary-hover-bg + rounded-md` |
| Glass Panel | `.glass-panel` 类：`rgba white/dark bg + backdrop-blur(20px)` |
| Feature Card | `.feature-card` 类：hover 时 `scale(1.02) + shadow elevation` |
| Tag/Badge | `tag-bg + tag-text + tag-border + rounded` |
| Kbd | `kbd-bg + kbd-border + kbd-text + rounded-md` |
| Toast | `toast-bg + backdrop-blur-xl + toast-border + rounded-xl + shadow-lg` |

---

## 五、技术栈概览（可用于 Landing Page "Built With" 板块）

| 层 | 技术 |
|----|------|
| 框架 | Tauri v2 (Rust + WebView) |
| 前端 | React 19 + TypeScript + Vite |
| 样式 | Tailwind CSS v4 (pure CSS tokens) |
| 动画 | Framer Motion v12 |
| 状态管理 | Zustand |
| STT 引擎 | whisper-rs (Metal GPU) + sherpa-onnx |
| LLM | OpenAI API compatible (GPT-4o-mini default) |
| 音频 | cpal (capture) + rubato (resample) |
| 粘贴 | arboard (clipboard) + AppleScript (Cmd+V) |
| 平台 | macOS 11+ (Apple Silicon recommended) |

---

## 六、竞品参考 & 差异化

### 竞品 Slogans

| 产品 | Slogan |
|------|--------|
| SuperWhisper | "Just speak. Write faster." |
| Wispr Flow | "Don't type, just speak" |
| WhisperKey | "Speak naturally. Get perfect text." |
| Utter | "Speak Naturally. Write Perfectly." |
| MacWhisper | "Your private transcription assistant" |

### Input 0 差异化优势

| 维度 | Input 0 | 竞品常见方案 |
|------|---------|-------------|
| STT 引擎 | 四大引擎（Whisper + SenseVoice + Paraformer + Moonshine），9 个模型，按语言智能推荐 | 通常只有 Whisper |
| 中文优化 | 40+ 拼音→技术术语纠错 + 简繁体引导 | 无或需手动修正 |
| 文本优化 | LLM 驱动 + 10 条历史上下文 + 自定义词汇库 | 简单后处理或无 |
| 隐私 | STT 完全本地，仅文本优化可选用外部 LLM | 部分产品需云端 STT |
| 粘贴 | 自动粘贴到当前输入框 | 多数需手动粘贴 |
| 模型管理 | 按需下载 + 一键切换 + 智能推荐 | 通常需手动下载配置 |
| 价格 | 免费 + 自带 API Key | 多数订阅制 |

---

## 七、Landing Page 结构建议

```
1. Hero Section
   - 主标题：Slogan（如 "Zero typing. Pure voice."）
   - 副标题：一句话功能描述
   - CTA 按钮："Download for macOS" / "Get Started Free"
   - Hero 动画/截图：Overlay 录音效果 GIF

2. Features Grid
   - 6-8 个功能卡片（参考第三节的 3.1-3.10）
   - 每个卡片：图标 + 标题 + 一句描述

3. How It Works
   - 三步流程图：
     ① 按住快捷键，开始说话
     ② 松开，AI 转录 + 优化
     ③ 文字自动粘贴到输入框

4. Privacy & Tech
   - 强调本地 STT
   - 技术栈展示（Built with Rust + Metal GPU）

5. Language Support
   - 多语言支持列表 + 智能模型推荐

6. Download CTA
   - macOS 下载按钮
   - 系统要求（macOS 11+, Apple Silicon recommended）

7. Footer
   - CC BY-NC 4.0 License
   - GitHub Link
```

