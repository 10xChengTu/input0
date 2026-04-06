# LLM 文本纠错 Prompt 优化 + 历史上下文

## 状态：已完成 ✅

## 需求描述

优化语音转文字后通过 LLM 进行文本纠错和优化的提示词（Prompt），具体包括：

1. **技术名词同音纠错** — 识别中文语音转写中被误转写为无意义字符拼接的英文技术名词（如"瑞嗯特"→"React"），在 prompt 中加入示例引导提高识别准确度。
2. **历史上下文** — 将用户最近 10 条转录历史作为上下文传给 LLM，提升语境理解和纠错准确度。

## 技术方案

### Prompt 架构

替换原有的单行硬编码 prompt，改为 `build_system_prompt(language)` 动态生成语言感知的系统 prompt：

- **核心规则**：去除填充词、修正语法、保持原意和语言、仅返回纠正后文本
- **zh / en / auto 路径**：均包含 40+ 中文拼音→英文技术名词映射表 + 检测策略 + 3 个完整 few-shot 输入→输出示例
  - `en` 路径额外包含 code-switching 指引，当说话者切换到中文时应用拼音纠正规则
  - `zh` 路径包含简繁体保持指引
- **上下文使用指引**：指导 LLM 如何利用历史上下文进行话题一致性维护和歧义消解

### 历史上下文集成

**方案选择**：文件持久化 — `history.json` 存储在应用配置目录 (`config_dir()/history.json`)。

- `HistoryEntry { original, corrected }` — 同时保留 STT 原文和 LLM 优化结果
- `build_context_message(history)` — 将最近 10 条历史格式化为 `STT: ... → Corrected: ...` 格式的上下文消息
- `history::load_history()` — 从文件加载历史，任何错误打日志（`log::warn`）并返回空 Vec（优雅降级）
- `history::save_history()` — 原子写入（temp file + rename）保存到文件，自动截断至最多 10 条
- Pipeline 自动回写：`process_audio()` 在 LLM 优化成功后通过 `history::append_entry()` 写入历史文件，FIFO 保持最多 10 条
- 历史跨应用重启持久化
- 消息架构：`[system_prompt, context_message(user role, 可选), user_message]`
  - 上下文消息使用 `user` role 而非 `system` role，因为历史内容为用户产生的不可信数据，提升到 system 级别会增加 prompt injection 风险

**注意**：
- `commands/llm.rs` 的独立 IPC 接口现通过 `history::load_history()` 加载真实历史上下文。

### 术语表覆盖

包含通用技术栈（React/API/JSON/TypeScript/Docker/Kubernetes 等）和本项目特定技术（Tauri/Vite/Zustand/Tailwind/Whisper/pnpm 等），共 40+ 条映射。

## 改动文件

| 文件 | 改动 |
|------|------|
| `src-tauri/src/llm/client.rs` | 移除硬编码 SYSTEM_PROMPT；新增 `HistoryEntry` 结构体、`build_system_prompt()`（含 40+ 术语映射 + 3 个 few-shot 示例，zh/en/auto 全路径均包含拼音表）、`build_context_message()`（user role）；`optimize_text()` 签名新增 `history` 参数；`LlmClient::new()` 返回 `Result<Self, AppError>` 替代 `.expect()`；`ChatMessage` 改为 `pub(crate)` 可见性 |
| `src-tauri/src/llm/tests.rs` | 更新所有 `optimize_text` 调用传 `&[]`；新增 10+ 测试覆盖 prompt 生成、上下文构建、历史截断、请求体结构 |
| `src-tauri/src/history.rs` | 新增文件持久化模块：`load_history()`（损坏文件打日志）、`save_history()`（原子写入 temp+rename） + 13 个单元测试 |
| `src-tauri/src/pipeline.rs` | 替换 `ManagedHistory` 为 `history::load_history()` / `history::append_entry()` 文件读写；移除 `ManagedHistory` 类型和 `new_managed_history()`；修复 `.unwrap()` |
| `src-tauri/src/lib.rs` | 新增 `mod history`；移除 `ManagedHistory` managed state 注册 |
| `src-tauri/src/commands/llm.rs` | `optimize_text` 调用更新为传入 `history::load_history()` 加载的真实历史上下文 |

## 验证结果

- `cargo test --lib`: 126 passed, 0 failed, 8 ignored ✅
- `pnpm build`: TypeScript 类型检查 + Vite 构建成功 ✅

## 后续优化方向

1. **用户自定义术语词典** — 允许用户在 Settings 页面添加自定义拼音→术语映射，注入到 prompt
2. ~~**历史持久化**~~ — ✅ 已完成：后端历史通过 `history.json` 文件持久化，支持跨会话上下文
3. **领域自适应 prompt** — 根据历史检测用户讨论领域，动态调整 prompt 侧重点
4. **置信度跳过** — STT 置信度高时跳过 LLM 优化，降低延迟
5. **流式 LLM 响应** — SSE 流式输出，改善体感速度
