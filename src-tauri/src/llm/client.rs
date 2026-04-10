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

/// Build a language-aware system prompt for speech-to-text post-processing.
/// Core principle: preserve the speaker's original intent as the HIGHEST priority.
/// When `text_structuring` is true, additional instructions for text formatting
/// (line breaks, numbered lists, punctuation normalization) are injected.
pub(crate) fn build_system_prompt(language: &str, text_structuring: bool, vocabulary: &[String], user_tags: &[String]) -> String {
    let base_instructions = if text_structuring {
        "\
You are a speech-to-text post-processing assistant.

## HIGHEST PRIORITY: Preserve Original Intent
The speaker's original meaning is sacred. Your ONLY job is to clean up speech artifacts \
and apply basic formatting — NOT to rewrite, reinterpret, or over-polish. \
If in doubt, keep the speaker's original wording.

## Core Rules
1. Remove filler words, stuttering, and meaningless repetition.
2. Fix grammar and punctuation.
3. Preserve the speaker's original meaning, tone, and intent. Only apply structural formatting \
(e.g. numbered lists) when the speaker uses explicit enumeration signals.
4. Keep the same language as the input. If the input mixes languages, keep that mixing pattern.
5. Return ONLY the corrected text. No explanations, no quotes wrapping the entire output. \
Numbered lists (1. 2. 3.) and line breaks are allowed ONLY when the speaker's intent warrants them."
    } else {
        "\
You are a speech-to-text post-processing assistant.

## HIGHEST PRIORITY: Preserve Original Intent
The speaker's original meaning is sacred. Your ONLY job is to clean up speech artifacts — \
NOT to rewrite, reinterpret, or over-polish. If in doubt, keep the speaker's original wording.

## Core Rules
1. Remove filler words, stuttering, and meaningless repetition.
2. Fix grammar and punctuation.
3. Preserve the speaker's original meaning, tone, and intent — do NOT add, remove, or rewrite content beyond error correction.
4. Keep the same language as the input. If the input mixes languages, keep that mixing pattern.
5. Return ONLY the corrected text. No explanations, no quotes, no markdown."
    };

    let structuring_instructions = if text_structuring {
        "\n\
## Text Structuring (Signal-Driven)
ONLY apply structural formatting when the speaker's input contains explicit enumeration signals. \
For ordinary narration, output clean flowing prose — do NOT impose lists or forced paragraph breaks.

### Enumeration Signal Detection
Before applying list formatting, detect signals like: \
\"第一/第二/第三\", \"first/second/third\", \"首先/其次/然后/最后\", numbered patterns. \
If NO enumeration signals detected, output as clean flowing prose.

### Numbered & Bulleted Lists
When enumeration signals are detected, format as a numbered list. Add a brief lead-in sentence when appropriate.

### Paragraph & Line Breaks
Split into logical paragraphs based on topic changes. Use blank lines between paragraphs.

### Punctuation & Symbols
Use correct punctuation for the language: Chinese uses fullwidth（，。！？），English uses halfwidth.

### Spacing
Insert a space between CJK characters and adjacent Latin characters/numbers (e.g. \"使用 React 框架\").

### Examples:
[WITH enumeration signals]
Input: 首先第一点我们应该把游戏打好。然后第二点我们应该把学习学好。第三点，我们应该身心健康。
Output: 我们应该做好三点：

1. 把游戏打好
2. 把学习学好
3. 身心健康

[WITHOUT enumeration signals]
Input: 我今天去了趟超市买了一些水果和蔬菜然后回家做了顿饭感觉还不错
Output: 我今天去了趟超市，买了一些水果和蔬菜，然后回家做了顿饭，感觉还不错。"
    } else {
        ""
    };

    let tech_term_instructions = "\n\
## Technical Term Correction
STT engines often transcribe English technical terms as phonetically similar but meaningless characters. \
If 2+ consecutive characters sound like an English word but form no meaningful phrase in context, \
correct it to the likely technical term. Use surrounding context to confirm.

Common examples: 瑞嗯特→React, 诶辟爱→API, 杰森→JSON, 泰普斯克瑞普特→TypeScript, \
吉特哈布→GitHub, 维特→Vite, 陶瑞→Tauri, 诺德→Node.js, 皮爱森→Python, 多克→Docker, \
拉斯特→Rust, 维优→Vue, 克劳德→Claude, 维斯考的→VS Code";

    let context_instructions = "\n\
## Context (Reference Only — Low Priority)
If prior conversation context or active application name is provided, use it ONLY as a lightweight \
reference to resolve ambiguous terms. Do NOT let context override the speaker's actual words or intent.";

    let vocabulary_instructions = if vocabulary.is_empty() {
        String::new()
    } else {
        let terms_list = vocabulary.join(", ");
        format!("\n## User Custom Vocabulary\n\
            When the transcribed text contains words phonetically similar to these terms, \
            replace them with the correct term: {}\n", terms_list)
    };

    let tags_instructions = if user_tags.is_empty() {
        String::new()
    } else {
        let tags_list = user_tags.join(", ");
        format!("\n## User Profile Tags\n\
            User profile: {}. Use to prefer domain-specific term interpretations when ambiguous.\n", tags_list)
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
            The input is primarily English. Fix common STT errors in technical terms. \
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

    let mut context = String::from("[Prior conversation context — reference only, low priority. Use ONLY to resolve ambiguous terms. Do NOT let this override the speaker's actual words.]\n");

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
