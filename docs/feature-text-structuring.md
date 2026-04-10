# 文本结构化优化

## 状态：已完成 ✅

## 需求描述

在 LLM 文本优化阶段，新增文本结构化处理能力，通过可选开关控制。主要包括：

1. **文本结构化** — 实现正确的换行、空行与空格；处理引号、字符串符号及各类标点符号；当说话者列举多个并列项时自动生成编号列表格式，增强整段文本可读性。
2. **内容精简与术语校正增强** — 去除多余语气词（已有基础，本次增强）；对 ASR 识别出的无意义但发音与特定技术名词/语境相似的文本，提取并还原为正确的专业术语（已有 40+ 映射表，本次在 prompt 中强化指引）。

## 设计决策

- **可选开关**：在 Settings 中提供 toggle，用户可按需控制是否启用文本结构化。默认关闭。
  - 理由：语音输入场景多样 — 聊天框中不需要结构化换行，文档/笔记中则很有价值。
- **实现方式**：纯 prompt 工程，在 `build_system_prompt()` 中条件注入结构化指令块，无需改动 LLM 调用逻辑。
- **信号驱动策略（v2 调整）**：仅当用户表达中包含明确结构化信号（序数词、列举词、数字编号等）时才应用格式化，对普通叙述保持自然文本流，避免过度结构化导致表达失真。

## 技术方案

### 配置层

- `AppConfig` 新增 `text_structuring: bool` 字段，`#[serde(default)]` 默认 `false`
- `update_field` 支持 `"text_structuring"` 字段（值为 `"true"` / `"false"`）
- 前端 `settings-store` 同步新增 `textStructuring` 状态

### Prompt 层

- `build_system_prompt(language, text_structuring)` 签名新增 `text_structuring: bool` 参数
- 当 `text_structuring = true` 时，在 Core Rules 之后、Technical Term Correction 之前插入 Text Structuring 指令块：
  - **信号驱动**：仅在检测到明确列举信号（序数词、列举词、数字编号、平行标记）时才应用列表格式化
  - 列表格式化：检测到列举信号时自动生成编号列表
  - 标点修正：引号配对、中英文标点正确使用
  - 中英文间距：英文单词与中文之间加空格
  - 空白清理：去除多余空格，段落间合理空行
  - **反面约束**：无列举信号的普通叙述保持自然文本流，不做强制结构化
- 当 `text_structuring = false` 时，保持现有行为（单段纯文本输出）

### Pipeline 层

- `optimize_text()` 签名新增 `text_structuring: bool` 参数
- `process_audio()` 从 config 读取 `text_structuring` 传入 `optimize_text()`
- `commands/llm.rs` 的独立 IPC 接口同步更新

### 前端 UI 层

- SettingsPage 的 Voice Settings section 新增 toggle 开关行
- i18n 新增对应翻译 key

## 改动文件

| 文件 | 改动 |
|------|------|
| `src-tauri/src/config/mod.rs` | `AppConfig` 新增 `text_structuring: bool`；`update_field` 新增分支 |
| `src-tauri/src/llm/client.rs` | `build_system_prompt()` 新增 `text_structuring` 参数 + 条件注入指令块；`optimize_text()` 新增参数 |
| `src-tauri/src/llm/tests.rs` | 新增 text_structuring prompt 生成测试 |
| `src-tauri/src/pipeline.rs` | `process_audio()` 读取 config.text_structuring 传入 optimize_text |
| `src-tauri/src/commands/llm.rs` | 更新 optimize_text 调用 |
| `src/stores/settings-store.ts` | 新增 textStructuring 状态 + load/save 同步 |
| `src/i18n/types.ts` | 新增翻译 key |
| `src/i18n/zh.ts` | 新增中文翻译 |
| `src/i18n/en.ts` | 新增英文翻译 |
| `src/components/SettingsPage.tsx` | 新增 toggle UI |

## 验证计划

- `cargo test --lib`: 所有测试通过 ✅ (147 passed, 0 failed)
- `pnpm build`: TypeScript 类型检查 + Vite 构建成功 ✅
