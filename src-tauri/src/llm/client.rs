use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_MODEL: &str = "gpt-4o-mini";
const REQUEST_TIMEOUT_SECS: u64 = 30;
const MAX_HISTORY_CONTEXT: usize = 10;

/// A completed transcription entry used as conversation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Raw STT transcription (before LLM optimization).
    pub original: String,
    /// LLM-optimized text (the final corrected result).
    pub corrected: String,
}

/// Build a language-aware system prompt with few-shot examples for
/// phonetic technical-term correction.
/// When `text_structuring` is true, additional instructions for text formatting
/// (line breaks, numbered lists, punctuation normalization) are injected.
pub(crate) fn build_system_prompt(language: &str, text_structuring: bool, vocabulary: &[String], user_tags: &[String]) -> String {
    let base_instructions = if text_structuring {
        "\
You are a speech-to-text post-processing assistant. Your job is to clean up \
raw transcriptions and produce polished, accurate, readable text.

## Core Rules
1. Remove filler words, stuttering, and meaningless repetition.
2. Fix grammar, punctuation, and sentence structure.
3. Preserve the speaker's original meaning, tone, and intent. Only apply structural formatting \
(e.g. numbered lists) when the speaker uses explicit enumeration signals (ordinal words, sequential markers). \
For ordinary narration, keep natural text flow — do NOT force structure onto conversational prose.
4. Keep the same language as the input. If the input mixes languages (e.g. Chinese with English terms), keep that mixing pattern.
5. Return ONLY the corrected text. No explanations, no quotes wrapping the entire output. \
Numbered lists (1. 2. 3.) and line breaks are allowed ONLY when the speaker's intent warrants them."
    } else {
        "\
You are a speech-to-text post-processing assistant. Your job is to clean up \
raw transcriptions and produce polished, accurate text.

## Core Rules
1. Remove filler words, stuttering, and meaningless repetition.
2. Fix grammar, punctuation, and sentence structure.
3. Preserve the speaker's original meaning, tone, and intent — do NOT add, remove, or rewrite content beyond error correction.
4. Keep the same language as the input. If the input mixes languages (e.g. Chinese with English terms), keep that mixing pattern.
5. Return ONLY the corrected text. No explanations, no quotes, no markdown."
    };

    let structuring_instructions = if text_structuring {
        "\n\
## Text Structuring (Signal-Driven)
ONLY apply structural formatting when the speaker's input contains explicit enumeration signals. \
For ordinary narration without such signals, output clean flowing prose — do NOT impose lists, \
bullet points, or forced paragraph breaks.

### Enumeration Signal Detection (PREREQUISITE — check BEFORE formatting)
Before applying ANY list formatting, you MUST detect at least one of these signals in the input:
- Ordinal words: \"第一/第二/第三\", \"first/second/third\", \"firstly/secondly/thirdly\"
- Sequential markers: \"首先/其次/然后/最后\", \"one/two/three\"
- Numbered patterns: \"第一点/第二点/第三点\", \"1. 2. 3.\"
- Explicit parallel markers: \"一个是…另一个是…\", \"one is…the other is…\"
If NO enumeration signal is detected, output as clean flowing prose — even if the text mentions multiple things.

### Numbered & Bulleted Lists (ONLY when signals detected)
- When enumeration signals are detected, format the enumerated items as a numbered list.
- Extract the core content of each item — remove redundant framing words \
(e.g. \"第一点我们应该\" → just the action) to make the list clean and scannable.
- Add a brief lead-in sentence with a colon before the list when appropriate.
- Use numbered lists (1. 2. 3.) for sequential or ordered items.

### Paragraph & Line Breaks
- Split text into logical paragraphs based on topic changes or natural pauses.
- Use a blank line between paragraphs.
- Do NOT break a single continuous thought into multiple paragraphs.
- When a list is detected, do NOT simply add line breaks between sentences — convert to a proper numbered list instead.

### Punctuation & Symbols
- Ensure proper pairing of quotation marks (Chinese「」/\u{201C}\u{201D} or English \"\").
- Use correct punctuation for the language: Chinese uses fullwidth punctuation（，。！？、：；）, English uses halfwidth.
- Fix misplaced or missing punctuation marks.

### Spacing
- Insert a space between CJK characters and adjacent Latin characters/numbers (e.g. \"使用 React 框架\" not \"使用React框架\").
- Remove excessive spaces while preserving intentional spacing.

### Few-shot examples (input → expected output):

[WITH enumeration signals → apply list formatting]

Input: 我觉得这个项目需要做三件事情首先是把API接口设计好其次是完成前端页面最后是写测试用例
Output: 我觉得这个项目需要做三件事情：

1. 把 API 接口设计好
2. 完成前端页面
3. 写测试用例

Input: 首先第一点我们应该把游戏打好。然后第二点我们应该把学习学好。第三点，我们应该身心健康。
Output: 我们应该做好三点：

1. 把游戏打好
2. 把学习学好
3. 身心健康

Input: 今天开会讨论了两个问题一个是关于发布流程的优化另一个是代码review的规范然后大家觉得应该先把CI CD流程搞好
Output: 今天开会讨论了两个问题：

1. 关于发布流程的优化
2. 代码 review 的规范

然后大家觉得应该先把 CI/CD 流程搞好。

Input: 第一要保证代码质量。第二要按时交付。第三要写好文档。第四要做好沟通。
Output: 需要做到以下几点：

1. 保证代码质量
2. 按时交付
3. 写好文档
4. 做好沟通

[WITHOUT enumeration signals → keep as natural prose, do NOT force structure]

Input: 我今天去了趟超市买了一些水果和蔬菜然后回家做了顿饭感觉还不错
Output: 我今天去了趟超市，买了一些水果和蔬菜，然后回家做了顿饭，感觉还不错。

Input: 这个项目的进展很顺利我们已经完成了大部分的功能开发下周准备开始测试
Output: 这个项目的进展很顺利，我们已经完成了大部分的功能开发，下周准备开始测试。

Input: 昨天和产品经理开了个会他说用户反馈这个功能不太好用需要优化一下交互体验我觉得可以先从按钮布局入手
Output: 昨天和产品经理开了个会，他说用户反馈这个功能不太好用，需要优化一下交互体验。我觉得可以先从按钮布局入手。"
    } else {
        ""
    };

    let tech_term_instructions = "\n\
## Technical Term Correction (CRITICAL)
Speech-to-text engines often transcribe English technical terms as phonetically similar \
but meaningless characters — especially when the speaker is using Chinese mixed with \
English jargon. You MUST detect and fix these.

### Common patterns to watch for:
| Misheard (phonetic) | Correct |
|---|---|
| 瑞嗯特 / 瑞艾克特 | React |
| 诶辟爱 / 爱批挨 | API |
| 杰森 (tech context) | JSON |
| 吉特 | Git |
| 吉特哈布 | GitHub |
| 泰普斯克瑞普特 | TypeScript |
| 贾瓦斯克瑞普特 | JavaScript |
| 奈克斯特 | Next.js |
| 诺德 | Node.js |
| 皮爱森 / 派森 | Python |
| 多科 / 多克 / 道克 | Docker |
| 库伯奈提斯 / 库博奈提斯 | Kubernetes |
| 拉斯特 | Rust |
| 维优 | Vue |
| 安归拉 | Angular |
| 斯维尔特 | Svelte |
| 开特GP提 | ChatGPT |
| 欧奥斯 / 欧森 | OAuth |
| 瑞迪斯 | Redis |
| 蒙哥 / 蒙哥DB | MongoDB |
| 帕斯特格瑞斯 | PostgreSQL |
| 兰姆达 | Lambda |
| 魏尔赛尔 | Vercel |
| 耐特利法 | Netlify |
| 斯普林 / 斯普瑞恩 | Spring |
| 卡夫卡 | Kafka |
| 伊拉斯提克 | Elasticsearch |
| 格拉夫QL | GraphQL |
| 批耳 | PR |
| 西爱西地 | CI/CD |
| 克劳德 | Claude |
| 欧拉马 | Ollama |
| 维斯考的 / VS考的 | VS Code |
| 陶瑞 / 套瑞 | Tauri |
| 维特 | Vite |
| 祖斯坦德 / 足斯坦 | Zustand |
| 泰尔温 / 台尔温德 | Tailwind |
| 维斯珀 / 威斯珀 | Whisper |
| 皮恩皮艾姆 | pnpm |
| 韦伯帕克 / 维伯帕克 | Webpack |
| 伊艾斯林特 | ESLint |
| 普瑞提尔 | Prettier |

### Detection strategy:
- If 2+ consecutive characters sound like an English word but form no meaningful Chinese phrase, treat it as a phonetic transcription and correct it.
- Use surrounding context to confirm: e.g. \"我在用瑞嗯特写组件\" → \"我在用 React 写组件\".
- When uncertain, prefer the technical term interpretation in a technical context.

### Few-shot examples (input → expected output):
Input: 我今天在用瑞嗯特和泰普斯克瑞普特写了一个新的组件，然后用维特来打包
Output: 我今天在用 React 和 TypeScript 写了一个新的组件，然后用 Vite 来打包

Input: 那个诶辟爱返回的杰森数据格式不对，我得看看后端诺德那边的代码
Output: 那个 API 返回的 JSON 数据格式不对，我得看看后端 Node.js 那边的代码

Input: 我把代码推到吉特哈布上了，然后建了一个PR等你review
Output: 我把代码推到 GitHub 上了，然后建了一个 PR 等你 review";

    let context_instructions = "\n\
## Using Prior Context
If prior conversation context is provided, use it to:
- Maintain topic consistency (e.g. if prior messages discussed a framework, new phonetic mentions likely refer to the same framework).
- Resolve ambiguous terms using surrounding context.
- Match terminology style the speaker has been using.
- If the active application name is provided, use it as a lightweight domain signal \
(e.g. code editors like VS Code or Xcode suggest technical/programming context; \
chat apps like Slack or Discord suggest conversational context; \
writing apps like Notion or Google Docs suggest prose context).
Do NOT repeat or reference the context in your output. It is for your understanding only.";

    let vocabulary_instructions = if vocabulary.is_empty() {
        String::new()
    } else {
        let terms_list = vocabulary.join(", ");
        format!("\n## User Custom Vocabulary (HIGHEST PRIORITY)\n\
            The user has specified these terms as important vocabulary. When the transcribed text \
            contains words that sound phonetically similar to any of these terms but are meaningless \
            or contextually wrong, you MUST replace them with the correct term from this list. \
            Use surrounding context and phonetic similarity to determine matches.\n\n\
            Terms: {}\n", terms_list)
    };

    let tags_instructions = if user_tags.is_empty() {
        String::new()
    } else {
        let tags_list = user_tags.join(", ");
        format!("\n## User Profile Tags\n\
            The user has specified the following profile tags that describe their profession, \
            interests, and work domains: {}.\n\
            Use this information to:\n\
            - Prefer domain-specific term interpretations when ambiguous \
            (e.g. if tagged \"developer\", phonetic gibberish in tech context is more likely a programming term)\n\
            - Apply appropriate jargon and terminology for their field\n\
            - Adjust formality level based on their work context\n\
            This is a persistent user preference, not a per-message instruction.\n", tags_list)
    };

    match language {
        "zh" => format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n\n## Language Note\n\
            The input is primarily Chinese. Pay special attention to Chinese phonetic \
            transcriptions of English technical terms. Preserve the Chinese variant used by the speaker \
            (simplified or traditional) — do not convert between them.",
            base_instructions, structuring_instructions, tech_term_instructions, context_instructions, vocabulary_instructions, tags_instructions
        ),
        "en" => format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n\n## Language Note\n\
            The input is primarily English. Fix common STT errors in technical terms: \
            misheard API names, framework names, programming language names, etc. \
            Use standard capitalization (e.g. \"JavaScript\" not \"javascript\"). \
            If the speaker code-switches into Chinese, apply the phonetic correction rules above.",
            base_instructions, structuring_instructions, tech_term_instructions, context_instructions, vocabulary_instructions, tags_instructions
        ),
        _ => {
            // "auto" or any other language code — include full instructions
            format!(
                "{}\n{}\n{}\n{}\n{}\n{}\n\n## Language Note\n\
                Auto-detect the language. If the input contains Chinese mixed with English \
                technical terms, apply the phonetic correction rules above.",
                base_instructions, structuring_instructions, tech_term_instructions, context_instructions, vocabulary_instructions, tags_instructions
            )
        }
    }
}

/// Build an optional context message from recent history entries.
/// Returns `None` if history is empty.
pub(crate) fn build_context_message(history: &[HistoryEntry], source_app: Option<&str>) -> Option<ChatMessage> {
    let has_history = !history.is_empty();
    let has_app = source_app.is_some();

    if !has_history && !has_app {
        return None;
    }

    let mut context = String::from("[Prior conversation context for reference — do NOT repeat this content, use it only to understand the ongoing topic and the speaker's speech patterns]\n");

    if let Some(app) = source_app {
        context.push_str(&format!("[Active application: {}]\n", app));
    }

    if has_history {
        let skip = history.len().saturating_sub(MAX_HISTORY_CONTEXT);
        let entries: Vec<&HistoryEntry> = history.iter().skip(skip).collect();

        for (i, entry) in entries.iter().enumerate() {
            context.push_str(&format!("{}. STT: {} → Corrected: {}\n", i + 1, entry.original, entry.corrected));
        }
    }

    Some(ChatMessage {
        role: "user".to_string(),
        content: context,
    })
}

#[derive(Serialize)]
pub(crate) struct ChatMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct ChatResponseChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Option<Vec<ChatResponseChoice>>,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: Option<String>,
}

#[derive(Deserialize)]
struct ApiErrorBody {
    error: Option<ApiErrorDetail>,
}

fn extract_api_error(status: reqwest::StatusCode, body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<ApiErrorBody>(body) {
        if let Some(detail) = parsed.error {
            if let Some(msg) = detail.message {
                return msg;
            }
        }
    }
    format!("API request failed (HTTP {})", status.as_u16())
}

pub struct LlmClient {
    api_key: String,
    base_url: String,
    pub(crate) model: String,
    http_client: reqwest::Client,
}

impl LlmClient {
    pub fn new(api_key: String, base_url: String, model: Option<String>) -> Result<Self, AppError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| AppError::Llm(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            api_key,
            base_url,
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            http_client,
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// Ask the LLM whether a vocabulary entry is a valid/meaningful correction.
    /// Returns true if the LLM considers it a legitimate vocabulary entry.
    pub async fn validate_vocabulary(&self, original: &str, correct: &str) -> Result<bool, AppError> {
        let url = format!("{}/chat/completions", self.base_url);

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a vocabulary validation assistant. The user will provide a pair of terms: an 'original' (potentially misheard speech-to-text output) and a 'correct' (the intended word). \
                         Your job is to determine if this is a legitimate vocabulary correction — i.e., the 'correct' term is a real, meaningful word/phrase, and it's plausible that speech-to-text could produce the 'original' as a mishearing. \
                         Respond with ONLY 'yes' or 'no'. No explanations.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("Original: {}\nCorrect: {}", original, correct),
            },
        ];

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages,
        };

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::Llm(format!("Network error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("(failed to read body)"));
            return Err(AppError::Llm(extract_api_error(status, &body)));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AppError::Llm(format!("Failed to parse response: {}", e)))?;

        let choices = chat_response
            .choices
            .ok_or_else(|| AppError::Llm("Response missing 'choices' field".to_string()))?;

        if choices.is_empty() {
            return Err(AppError::Llm("Response contains empty 'choices' array".to_string()));
        }

        let answer = choices[0].message.content.trim().to_lowercase();
        Ok(answer.starts_with("yes"))
    }

    pub async fn test_connection(&self) -> Result<String, AppError> {
        let url = format!("{}/chat/completions", self.base_url);

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
        };

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::Llm(format!("Network error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("(failed to read body)"));
            return Err(AppError::Llm(extract_api_error(status, &body)));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AppError::Llm(format!("Failed to parse response: {}", e)))?;

        let choices = chat_response
            .choices
            .ok_or_else(|| AppError::Llm("Response missing 'choices' field".to_string()))?;

        if choices.is_empty() {
            return Err(AppError::Llm("Response contains empty 'choices' array".to_string()));
        }

        Ok(format!("Connected — model {} is working", self.model))
    }

    pub async fn optimize_text(&self, raw_text: &str, language: &str, history: &[HistoryEntry], text_structuring: bool, vocabulary: &[String], source_app: Option<&str>, user_tags: &[String]) -> Result<String, AppError> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: build_system_prompt(language, text_structuring, vocabulary, user_tags),
            },
        ];

        if let Some(ctx) = build_context_message(history, source_app) {
            messages.push(ctx);
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: raw_text.to_string(),
        });

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages,
        };

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::Llm(format!("Network error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("(failed to read body)"));
            return Err(AppError::Llm(extract_api_error(status, &body)));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AppError::Llm(format!("Failed to parse response JSON: {}", e)))?;

        let choices = chat_response
            .choices
            .ok_or_else(|| AppError::Llm("Response missing 'choices' field".to_string()))?;

        if choices.is_empty() {
            return Err(AppError::Llm("Response contains empty 'choices' array".to_string()));
        }

        let first_choice = choices
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Llm("Response contains empty 'choices' array".to_string()))?;

        Ok(first_choice.message.content)
    }
}
