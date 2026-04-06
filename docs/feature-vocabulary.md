# Feature: 词汇库（自定义词汇 + 自动学习 + Prompt 集成）

## 状态：已完成 ✅

## 需求概述

1. **手动管理**：用户在设置页面手动添加/删除自定义词汇（仅需输入正确词汇，无需原词）
2. **自动学习**：Pipeline 完成后展示转录结果，用户编辑修改后提交，系统自动 diff 检测词级替换，通过 LLM 验证后将正确词汇加入词汇库
3. **Prompt 集成**：词汇库内容注入到 LLM system prompt 中，标记为 HIGHEST PRIORITY，LLM 根据语境和发音相似度自动匹配词汇库中的正确词汇

## 技术方案

### 架构设计

```
Vec<String> (正确词汇列表)
    ↓
vocabulary.rs (JSON 持久化, Mutex 线程安全, 原子写入)
    ↓
commands/vocabulary.rs (4 个 IPC 命令)
    ↓
LLM Prompt 注入 (build_system_prompt → User Custom Vocabulary 段落，纯 terms 列表 + 发音推断指令)
    ↓
Pipeline (process_audio → load_vocabulary → optimize_text)

前端自动学习流程:
Pipeline Done → useTauriEvents setLastResult()
    → HomePage 纠正卡片 (预览/编辑模式)
    → detectReplacements (LCS word-diff)
    → validateAndAdd (LLM 验证，只存 correct 词汇)
    → 词汇库更新 + toast 通知
```

### 数据模型

```rust
// src-tauri/src/vocabulary.rs
// 词汇库存储为纯字符串列表，每个元素是一个正确词汇
// 用户只需提供正确词汇，LLM 根据语境和发音自行推断匹配
type Vocabulary = Vec<String>;
```

存储路径：`~/Library/Application Support/com.input0.dev/vocabulary.json`

存储格式：JSON 字符串数组，如 `["React", "TypeScript", "Kubernetes"]`

限制：最多 500 条，超出时保留最新的 500 条。

### 设计理由

为什么只存正确词汇而不存 (original, correct) 映射对：
- 用户不知道语音识别会把词错误识别成什么（每次可能不同）
- 同一个词可能被识别成多种不同的错误形式，只有发音相同
- LLM 通过语境 + 发音相似度自行推断，比固定映射更灵活准确

### 后端实现

#### 词汇库存储 (`src-tauri/src/vocabulary.rs`)

- `load_vocabulary() -> Vec<String>` — 从 JSON 文件加载，失败返回空 Vec
- `save_vocabulary(entries)` — 原子写入（先写 tmp 再 rename）
- `add_entry(term) -> bool` — 去重（相同词汇跳过，返回 false），Mutex 保护。返回 true 表示新增，false 表示已存在
- `remove_entry(term) -> bool` — 按词汇删除，返回是否存在

#### IPC 命令 (`src-tauri/src/commands/vocabulary.rs`)

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_vocabulary` | 无 | `Vec<String>` | 获取全部词汇 |
| `add_vocabulary_entry` | term | `bool` | 直接添加，返回 true=新增/false=已存在 |
| `remove_vocabulary_entry` | term | `bool` | 删除，返回是否存在 |
| `validate_and_add_vocabulary` | original, correct | `bool` | LLM 验证后添加 correct 词汇 |

#### LLM 验证 (`src-tauri/src/llm/client.rs`)

- `validate_vocabulary(original, correct)` — 调用 LLM 判断纠正是否合理（yes/no）
- 无 API Key 时返回 `Err(AppError::Llm(...))`，不跳过验证

#### Prompt 注入 (`src-tauri/src/llm/client.rs`)

`build_system_prompt(language, text_structuring, vocabulary)` 新增 vocabulary 参数（`&[String]`）：
- 非空时注入 `## User Custom Vocabulary (HIGHEST PRIORITY)` 段落
- 格式为纯 terms 列表，附发音推断指令：LLM 根据语境和发音相似度自行判断匹配
- 优先级高于内置技术术语纠正表

#### Pipeline 集成 (`src-tauri/src/pipeline.rs`)

`process_audio()` 中加载词汇库传递给 `optimize_text()`。

### 前端实现

#### 词汇库管理页面 (`src/components/VocabularyPage.tsx`)

- 添加表单：单个输入框输入正确词汇 → 调用 `add_vocabulary_entry`
- 列表展示：搜索过滤、删除操作
- 空状态：引导用户添加词汇

#### 自动学习 UI (`src/components/HomePage.tsx`)

- 纠正卡片：Pipeline 完成后在首页展示最近转录结果
- 预览模式：显示文本 + "编辑并提交" 按钮
- 编辑模式：textarea 预填优化后文本 + "提交纠正" 按钮
- 提交流程：diff → 逐个 validateAndAdd → toast 通知（区分成功/失败/已存在/无API Key）

#### 词级 Diff (`src/utils/word-diff.ts`)

- `tokenize(text)` — Unicode-aware 分词（`[\p{L}\p{N}]+` 等）
- `lcsLength(a, b)` — LCS 动态规划
- `detectReplacements(original, corrected)` — 返回 `WordReplacement[]`

#### 状态管理

- `recording-store.ts` — 新增 `lastTranscribedText`, `lastOptimizedText`, `setLastResult()`, `clearLastResult()`
- `vocabulary-store.ts` — Zustand store，`entries: string[]`，封装 4 个 IPC 调用
- `useTauriEvents.ts` — done 事件中调用 `setLastResult()`

#### 国际化 (`src/i18n/`)

- `types.ts` — vocabulary 和 home.correction 翻译类型
- `zh.ts` / `en.ts` — 完整中英文翻译

## 文件清单

### 新建文件

| 文件 | 说明 |
|------|------|
| `src-tauri/src/vocabulary.rs` | 词汇库后端存储模块 |
| `src-tauri/src/commands/vocabulary.rs` | 4 个 IPC 命令 |
| `src/stores/vocabulary-store.ts` | 前端 Zustand store |
| `src/components/VocabularyPage.tsx` | 词汇库管理页面 |
| `src/utils/word-diff.ts` | 词级 diff 工具（LCS） |

### 修改文件

| 文件 | 修改内容 |
|------|----------|
| `src-tauri/src/lib.rs` | 注册 `pub mod vocabulary` + 4 个 IPC 命令 |
| `src-tauri/src/commands/mod.rs` | 添加 `pub mod vocabulary` |
| `src-tauri/src/errors.rs` | 添加 `Vocabulary(String)` 错误变体 |
| `src-tauri/src/llm/client.rs` | `build_system_prompt` 新增 vocabulary 参数 + `validate_vocabulary` 方法 + `optimize_text` 新增 vocabulary 参数 |
| `src-tauri/src/llm/tests.rs` | 更新所有 `build_system_prompt` 和 `optimize_text` 调用签名 |
| `src-tauri/src/pipeline.rs` | `process_audio` 加载 vocabulary 传递给 `optimize_text` |
| `src-tauri/src/commands/llm.rs` | `optimize_text` 命令加载 vocabulary 传递 |
| `src/components/HomePage.tsx` | 自动学习纠正卡片 UI |
| `src/components/Sidebar.tsx` | vocabulary 导航项 |
| `src/pages/Settings.tsx` | VocabularyPage 路由挂载 + HomePage onToast prop |
| `src/stores/recording-store.ts` | 新增 last* 字段 |
| `src/hooks/useTauriEvents.ts` | done 事件保存 lastResult |
| `src/i18n/types.ts` | vocabulary + correction 翻译类型 |
| `src/i18n/zh.ts` | 中文翻译 |
| `src/i18n/en.ts` | 英文翻译 |
