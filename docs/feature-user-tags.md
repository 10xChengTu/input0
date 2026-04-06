# 用户偏好标签（User Preference Tags）

## 状态：已完成 ✅

## 需求描述

用户可在 Settings 页面选择/配置以下三类偏好标签：

1. **职业标签** — 开发者、设计师、产品经理等
2. **领域标签** — 感兴趣的领域（AI、前端、后端、数据分析等）
3. **工作空间标签** — 日常常用的工作场景/领域

这些标签在语音转文字的 LLM 文本优化阶段注入 system prompt，帮助 LLM 根据用户身份和领域偏好更精准地纠错（如优先识别特定领域的技术名词）。

## 技术方案

### 数据模型

```rust
// config/mod.rs - AppConfig 新增字段
#[serde(default)]
pub user_tags: Vec<String>,
```

- 使用 `Vec<String>` 存储，TOML 序列化为字符串数组
- `#[serde(default)]` 确保向后兼容（旧配置文件缺少该字段时默认为空数组）
- 前端以预定义标签 + 自定义标签的方式让用户选择

### 配置持久化

- **存储路径**：`~/Library/Application Support/com.input0.app/config.toml`
- **update_field 处理**：`"user_tags"` 分支接受 JSON 字符串（`serde_json::from_str` 解析），前端传 `JSON.stringify(tags)`

### LLM 注入

- **注入位置**：system prompt 中新增 `tags_instructions` 段落（类似 vocabulary_instructions 的模式）
- **优先级**：作为高优先级领域信号，帮助 LLM 在模糊语境中倾向用户领域内的术语解读

```
## User Profile Tags
The user has specified the following profile tags that describe their profession,
interests, and work domains: [tag1, tag2, tag3].
Use this information to:
- Prefer domain-specific term interpretations when ambiguous
- Apply appropriate jargon and terminology for their field
- Adjust formality level based on their work context
```

### Pipeline 数据流

```
process_audio()
  → config::load() → config.user_tags
  → optimize_text(..., &config.user_tags)
  → build_system_prompt(..., user_tags) → tags_instructions 段落
  → LLM system prompt 包含用户标签信息
```

user_tags 是 config 的一部分，直接从 config 读取即可，不需要通过 Pipeline/RecordedAudio 传递。

### 函数签名变更

```rust
// llm/client.rs
pub(crate) fn build_system_prompt(language: &str, text_structuring: bool, vocabulary: &[String], user_tags: &[String]) -> String
pub async fn optimize_text(&self, ..., user_tags: &[String]) -> Result<String, AppError>
```

### 前端 UI

在 Settings 页面新增「用户偏好标签」section，包含三组预定义标签（职业/领域/工作空间），用户点击 tag chip 切换选中/取消，选中态使用已有的 active tag 样式。

### 降级策略

- `user_tags` 为空时不注入任何标签指引到 system prompt
- 旧配置文件不含 `user_tags` 字段 → `serde(default)` 自动填充空数组
- 不影响任何现有功能

## 改动文件

| 文件 | 改动 |
|------|------|
| `src-tauri/src/config/mod.rs` | AppConfig 新增 `user_tags: Vec<String>` 字段 + Default + update_field 分支 |
| `src-tauri/src/llm/client.rs` | `build_system_prompt()` 新增 `user_tags` 参数 + tags_instructions 段落；`optimize_text()` 新增 `user_tags` 参数 |
| `src-tauri/src/pipeline.rs` | `process_audio()` 从 config 读取 user_tags 传入 optimize_text |
| `src-tauri/src/commands/llm.rs` | IPC `optimize_text` 调用传入 user_tags（从 config 读取） |
| `src-tauri/src/llm/tests.rs` | 更新所有 build_system_prompt/optimize_text 调用 + 新增 user_tags 测试 |
| `src/stores/settings-store.ts` | 新增 `userTags` 字段 + loadConfig/saveConfig 映射 |
| `src/i18n/types.ts` | 新增 settings.userTags* 类型定义 |
| `src/i18n/en.ts` | 新增英文翻译 |
| `src/i18n/zh.ts` | 新增中文翻译 |
| `src/components/SettingsPage.tsx` | 新增用户偏好标签选择 UI section |

## 新增测试

| 测试 | 覆盖场景 |
|------|----------|
| `test_system_prompt_with_user_tags` | user_tags 非空时 system prompt 包含标签信息 |
| `test_system_prompt_without_user_tags` | user_tags 为空时 system prompt 不包含标签段落 |
| `test_config_user_tags_default` | 旧配置文件缺少 user_tags 字段时默认空数组 |
| `test_update_field_user_tags` | update_field 接受 JSON 字符串正确解析为 Vec<String> |

## 验证结果

- `cargo test --lib`: ✅ 146 passed, 0 failed
- `pnpm build`: ✅ tsc + vite build 成功
