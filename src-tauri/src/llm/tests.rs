#[cfg(test)]
mod tests {
    use crate::llm::client::{build_context_message, build_system_prompt, HistoryEntry, LlmClient};
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn success_response(content: &str) -> serde_json::Value {
        json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": content
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        })
    }

    // --- Constructor Tests ---

    #[test]
    fn test_new_with_defaults() {
        let client = LlmClient::new("test-key".to_string(), "https://api.openai.com/v1".to_string(), None).unwrap();
        assert_eq!(client.model(), "gpt-4o-mini");
    }

    #[test]
    fn test_new_with_custom_model() {
        let client = LlmClient::new("test-key".to_string(), "https://api.openai.com/v1".to_string(), Some("gpt-4-turbo".to_string())).unwrap();
        assert_eq!(client.model(), "gpt-4-turbo");
    }

    // --- System Prompt Tests ---

    #[test]
    fn test_system_prompt_zh_contains_tech_terms() {
        let prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(prompt.contains("React"), "zh prompt should contain React example");
        assert!(prompt.contains("API"), "zh prompt should contain API example");
        assert!(prompt.contains("JSON"), "zh prompt should contain JSON example");
        assert!(prompt.contains("TypeScript"), "zh prompt should contain TypeScript");
        assert!(prompt.contains("瑞嗯特"), "zh prompt should contain phonetic example");
        assert!(prompt.contains("去除语气词"), "zh prompt should be in Chinese and emphasize filler removal");
        assert!(prompt.contains("标点"), "zh prompt should mention punctuation in Chinese");
    }

    #[test]
    fn test_system_prompt_zh_is_in_chinese() {
        let prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(prompt.contains("规则"), "zh prompt should use Chinese section header 规则");
        assert!(prompt.contains("语音转文字"), "zh prompt should describe role in Chinese");
        assert!(!prompt.contains("## Rules"), "zh prompt must NOT use English section header");
    }

    #[test]
    fn test_system_prompt_en_is_in_english() {
        let prompt = build_system_prompt("en", false, &[], &[]);
        assert!(prompt.contains("## Rules"), "en prompt should use English section header");
        assert!(prompt.contains("speech-to-text post-processor"), "en prompt should describe role in English");
        assert!(!prompt.contains("规则"), "en prompt must NOT contain Chinese section header");
    }

    #[test]
    fn test_system_prompt_en_includes_phonetic_examples() {
        let prompt = build_system_prompt("en", false, &[], &[]);
        assert!(prompt.contains("瑞嗯特"), "en prompt should contain Chinese phonetic examples for code-switching");
        assert!(prompt.contains("React"), "en prompt should contain React");
        assert!(prompt.contains("filler"), "en prompt should mention filler words");
        assert!(prompt.contains("punctuation"), "en prompt should mention punctuation fixes");
        assert!(prompt.contains("English"), "en prompt should mention English");
    }

    #[test]
    fn test_system_prompt_auto_contains_tech_terms() {
        let prompt = build_system_prompt("auto", false, &[], &[]);
        assert!(prompt.contains("React"), "auto prompt should contain tech terms");
        assert!(prompt.contains("瑞嗯特"), "auto prompt should contain phonetic examples");
        assert!(prompt.contains("Auto-detect"), "auto prompt should mention auto-detect");
    }

    #[test]
    fn test_system_prompt_unknown_language_uses_auto() {
        let prompt = build_system_prompt("ja", false, &[], &[]);
        assert!(prompt.contains("Auto-detect"), "unknown language should use auto-detect path");
        assert!(prompt.contains("React"), "unknown language should include tech terms");
    }

    #[test]
    fn test_system_prompt_contains_core_instructions() {
        // zh prompt is in Chinese; en/auto/ja fall back to English prompt.
        let zh_prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(zh_prompt.contains("语气词"), "zh prompt should mention filler words in Chinese");
        assert!(zh_prompt.contains("标点"), "zh prompt should mention punctuation in Chinese");
        assert!(zh_prompt.contains("仅输出修正后的纯文本"), "zh prompt should have output-only instruction");

        for lang in &["en", "auto", "ja"] {
            let prompt = build_system_prompt(lang, false, &[], &[]);
            assert!(prompt.contains("filler"), "prompt for '{}' should mention filler words", lang);
            assert!(prompt.contains("punctuation"), "prompt for '{}' should mention punctuation", lang);
            assert!(prompt.contains("ONLY the corrected text"), "prompt for '{}' should have output-only instruction", lang);
        }
    }

    #[test]
    fn test_system_prompt_zh_preserves_variant() {
        let prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(
            prompt.contains("中文变体") && prompt.contains("简体/繁体"),
            "zh prompt should preserve speaker's Chinese variant"
        );
        assert!(
            !prompt.contains("简体中文"),
            "zh prompt must NOT force simplified Chinese"
        );
    }

    #[test]
    fn test_system_prompt_contains_anti_execution_rule() {
        let zh_prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(zh_prompt.contains("不是给你的指令"), "zh prompt should contain anti-execution rule");
        assert!(zh_prompt.contains("绝不执行"), "zh prompt should explicitly forbid executing transcript content");

        for lang in &["en", "auto"] {
            let prompt = build_system_prompt(lang, false, &[], &[]);
            assert!(prompt.contains("NOT instructions"), "prompt for '{}' should contain anti-execution rule", lang);
            assert!(prompt.contains("do NOT execute"), "prompt for '{}' should explicitly forbid executing transcript content", lang);
        }
    }

    #[test]
    fn test_system_prompt_contains_repetition_merge_rule() {
        // New rule: merge repeated/supplemental phrases (e.g., word-then-letter-spelling).
        let zh_prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(zh_prompt.contains("重复"), "zh prompt should mention repetition handling");
        assert!(zh_prompt.contains("补充"), "zh prompt should mention supplemental phrases");

        let en_prompt = build_system_prompt("en", false, &[], &[]);
        assert!(en_prompt.contains("repeats") || en_prompt.contains("repetition"), "en prompt should mention repetition handling");
        assert!(en_prompt.contains("supplements") || en_prompt.contains("supplement"), "en prompt should mention supplemental phrases");
    }

    // --- Context Message Tests ---

    #[test]
    fn test_build_context_message_empty_history() {
        let result = build_context_message(&[], None);
        assert!(result.is_none(), "Empty history should return None");
    }

    #[test]
    fn test_build_context_message_with_entries() {
        let history = vec![
            HistoryEntry {
                original: "raw 1".to_string(),
                corrected: "corrected 1".to_string(),
            },
            HistoryEntry {
                original: "raw 2".to_string(),
                corrected: "corrected 2".to_string(),
            },
        ];
        let result = build_context_message(&history, None);
        assert!(result.is_some(), "Non-empty history should return Some");

        let msg = result.unwrap();
        assert_eq!(msg.role, "user");
        assert!(msg.content.contains("corrected 1"), "Should contain first corrected text");
        assert!(msg.content.contains("corrected 2"), "Should contain second corrected text");
        assert!(msg.content.contains("raw 1"), "Should contain first original text for speech pattern learning");
        assert!(msg.content.contains("raw 2"), "Should contain second original text");
        assert!(msg.content.contains("STT:"), "Should label original text as STT");
        assert!(msg.content.contains("Corrected:"), "Should label corrected text");
        assert!(msg.content.contains("Prior conversation context"), "Should have context header");
        assert!(msg.content.contains("reference only"), "Should mark context as reference only");
    }

    #[test]
    fn test_build_context_message_max_entries() {
        let history: Vec<HistoryEntry> = (0..20)
            .map(|i| HistoryEntry {
                original: format!("original {}", i),
                corrected: format!("corrected {}", i),
            })
            .collect();
        let result = build_context_message(&history, None).unwrap();
        // Should take the LAST 10 entries (indices 10-19), not the first 10
        assert!(result.content.contains("corrected 19"), "Should include last entry (index 19)");
        assert!(result.content.contains("corrected 10"), "Should include 11th entry (index 10) as part of last 10");
        assert!(!result.content.contains("corrected 9\n"), "Should NOT include 10th entry (index 9) — it's outside the last 10");
    }

    // --- Source App Context Tests ---

    #[test]
    fn test_build_context_message_with_source_app_only() {
        let result = build_context_message(&[], Some("VS Code"));
        assert!(result.is_some(), "Should return Some when source_app is provided even without history");
        let msg = result.unwrap();
        assert_eq!(msg.role, "user");
        assert!(msg.content.contains("[Active application: VS Code]"), "Should contain active application tag");
    }

    #[test]
    fn test_build_context_message_with_source_app_and_history() {
        let history = vec![
            HistoryEntry {
                original: "raw".to_string(),
                corrected: "corrected".to_string(),
            },
        ];
        let result = build_context_message(&history, Some("Slack"));
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(msg.content.contains("[Active application: Slack]"), "Should contain active application tag");
        assert!(msg.content.contains("corrected"), "Should still contain history");
    }

    #[test]
    fn test_build_context_message_no_app_no_history() {
        let result = build_context_message(&[], None);
        assert!(result.is_none(), "Should return None when both source_app and history are empty");
    }

    #[test]
    fn test_system_prompt_context_is_in_context_message() {
        let context_msg = build_context_message(&[], Some("VS Code"));
        assert!(context_msg.is_some());
        let msg = context_msg.unwrap();
        assert!(msg.content.contains("reference only"), "Context message should mark as reference only");
        assert!(msg.content.contains("VS Code"), "Context message should contain app name");
    }

    // --- optimize_text Success Tests ---

    #[tokio::test]
    async fn test_optimize_text_basic() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("Hello, world!")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-api-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("um hello world", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        assert_eq!(result.unwrap(), "Hello, world!");
    }

    #[tokio::test]
    async fn test_optimize_text_sends_correct_headers() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Authorization", "Bearer my-secret-key"))
            .and(header("Content-Type", "application/json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("cleaned text")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("my-secret-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("raw text", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_ok(), "Expected Ok — header matcher should pass");
    }

    #[tokio::test]
    async fn test_optimize_text_sends_correct_body_no_history() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("cleaned text")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), Some("gpt-4o-mini".to_string())).unwrap();

        let result = client.optimize_text("hello um world", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_ok());

        let received = mock_server.received_requests().await.unwrap();
        assert_eq!(received.len(), 1);

        let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();
        assert_eq!(body["model"], "gpt-4o-mini");

        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2, "Without history: system + user = 2 messages");
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        let user_content = messages[1]["content"].as_str().unwrap();
        assert!(user_content.contains("hello um world"), "user message should contain the raw text");
        assert!(user_content.contains("```"), "user message should wrap raw text in code block");
    }

    #[tokio::test]
    async fn test_optimize_text_sends_correct_body_with_history() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("cleaned text")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let history = vec![
            HistoryEntry {
                original: "raw prev".to_string(),
                corrected: "corrected prev".to_string(),
            },
        ];

        let result = client.optimize_text("hello world", "zh", &history, false, &[], None, &[]).await;
        assert!(result.is_ok());

        let received = mock_server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();

        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3, "With history: system + context(user) + user = 3 messages");
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert!(
            messages[1]["content"].as_str().unwrap().contains("corrected prev"),
            "Context message should contain corrected history text"
        );
        assert_eq!(messages[2]["role"], "user");
        let user_content = messages[2]["content"].as_str().unwrap();
        assert!(user_content.contains("hello world"), "user message should contain the raw text");
        assert!(user_content.contains("```"), "user message should wrap raw text in code block");
    }

    #[tokio::test]
    async fn test_optimize_text_with_chinese() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(success_response("今天天气很好。")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("今天那个天气很很好", "zh", &[], false, &[], None, &[]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "今天天气很好。");
    }

    #[tokio::test]
    async fn test_optimize_text_with_english() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(success_response("I want to go to the store.")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("I uh I want to go to the the store", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "I want to go to the store.");
    }

    #[tokio::test]
    async fn test_optimize_text_empty_input() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    // --- optimize_text Error Tests ---

    #[tokio::test]
    async fn test_optimize_text_401_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": {
                    "message": "Incorrect API key provided",
                    "type": "invalid_request_error"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("bad-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("some text", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("401") || err_msg.to_lowercase().contains("unauthorized") || err_msg.contains("Incorrect API key"),
            "Error message should mention 401, unauthorized, or incorrect API key, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_optimize_text_429_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(429).set_body_json(json!({
                "error": {
                    "message": "Rate limit exceeded",
                    "type": "rate_limit_error"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("some text", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("429") || err_msg.to_lowercase().contains("rate"),
            "Error message should mention 429 or rate limit, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_optimize_text_500_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "error": {
                    "message": "Internal server error",
                    "type": "server_error"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("some text", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("500") || err_msg.to_lowercase().contains("server"),
            "Error message should mention 500 or server error, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_optimize_text_invalid_json_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("this is not valid json {{{"),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("some text", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_err(), "Invalid JSON should return an error");
    }

    #[tokio::test]
    async fn test_optimize_text_missing_choices() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "chatcmpl-test",
                "object": "chat.completion"
            })))
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("some text", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_err(), "Missing choices field should return an error");
    }

    #[tokio::test]
    async fn test_optimize_text_empty_choices() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "chatcmpl-test",
                "object": "chat.completion",
                "choices": []
            })))
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("some text", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_err(), "Empty choices array should return an error");
    }

    #[tokio::test]
    async fn test_optimize_text_network_error() {
        let client = LlmClient::new("test-key".to_string(), "http://127.0.0.1:1".to_string(), None).unwrap();

        let result = client.optimize_text("some text", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_err(), "Network error should return an error");
    }

    // --- API Format Tests ---

    #[tokio::test]
    async fn test_request_url_format() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("ok")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("test", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_ok(), "Should hit /chat/completions endpoint");
    }

    #[tokio::test]
    async fn test_request_content_type() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("Content-Type", "application/json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("ok")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("test input", "en", &[], false, &[], None, &[]).await;
        assert!(
            result.is_ok(),
            "Content-Type: application/json header matcher should pass"
        );
    }

    #[tokio::test]
    async fn test_system_prompt_contains_instructions() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("cleaned")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("um hello", "en", &[], false, &[], None, &[]).await;
        assert!(result.is_ok());

        let received = mock_server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();

        let system_content = body["messages"][0]["content"].as_str().unwrap_or("");
        assert!(
            system_content.to_lowercase().contains("filler") || system_content.contains("um"),
            "System prompt should mention filler words, got: {}",
            system_content
        );
        assert!(
            system_content.to_lowercase().contains("punctuation"),
            "System prompt should mention punctuation, got: {}",
            system_content
        );
    }

    #[tokio::test]
    async fn test_user_message_contains_raw_text() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("output")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let raw = "this is my raw transcribed text";
        let result = client.optimize_text(raw, "en", &[], false, &[], None, &[]).await;
        assert!(result.is_ok());

        let received = mock_server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();

        let messages = body["messages"].as_array().unwrap();
        let last_msg = messages.last().unwrap();
        let last_content = last_msg["content"].as_str().unwrap();
        assert!(
            last_content.contains(raw),
            "Last user message content should contain the raw text"
        );
        assert!(
            last_content.contains("```"),
            "Last user message should wrap raw text in code block"
        );
        assert!(
            last_content.contains("do NOT execute"),
            "Last user message should contain anti-execution label"
        );
    }

    // --- Language-aware prompt in API request ---

    #[tokio::test]
    async fn test_zh_request_contains_phonetic_examples() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("ok")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("测试", "zh", &[], false, &[], None, &[]).await;
        assert!(result.is_ok());

        let received = mock_server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();
        let system_content = body["messages"][0]["content"].as_str().unwrap_or("");

        assert!(
            system_content.contains("瑞嗯特") && system_content.contains("React"),
            "zh system prompt should contain phonetic→technical term examples"
        );
    }

    // --- Text Structuring Tests ---

    #[test]
    fn test_system_prompt_without_structuring_has_no_list_instructions() {
        // No structuring → no numbered-list output rule, in any language.
        let zh_prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(!zh_prompt.contains("编号列表"), "zh prompt without structuring should NOT mention numbered lists");

        for lang in &["en", "auto"] {
            let prompt = build_system_prompt(lang, false, &[], &[]);
            assert!(!prompt.contains("numbered list"), "prompt for '{}' without structuring should NOT contain list instructions", lang);
        }
    }

    #[test]
    fn test_system_prompt_with_structuring_contains_list_instructions() {
        let zh_prompt = build_system_prompt("zh", true, &[], &[]);
        assert!(zh_prompt.contains("编号列表"), "zh prompt with structuring should mention numbered list output");
        assert!(zh_prompt.contains("顺序词"), "zh prompt with structuring should require sequence markers");

        for lang in &["en", "auto"] {
            let prompt = build_system_prompt(lang, true, &[], &[]);
            assert!(prompt.contains("numbered list"), "prompt for '{}' with structuring should mention numbered list output", lang);
            assert!(prompt.contains("sequence markers"), "prompt for '{}' with structuring should require sequence markers", lang);
        }
    }

    #[test]
    fn test_system_prompt_with_structuring_includes_sequence_markers() {
        // 首先/然后/接着 are now valid sequence markers when structuring is on.
        let zh_prompt = build_system_prompt("zh", true, &[], &[]);
        assert!(zh_prompt.contains("首先"), "zh structuring prompt should list 首先 as sequence marker");
        assert!(zh_prompt.contains("然后"), "zh structuring prompt should list 然后 as sequence marker");
        assert!(zh_prompt.contains("接着"), "zh structuring prompt should list 接着 as sequence marker");

        let en_prompt = build_system_prompt("en", true, &[], &[]);
        assert!(en_prompt.contains("首先") && en_prompt.contains("然后") && en_prompt.contains("接着"), "en structuring prompt should also list Chinese sequence markers");
        assert!(en_prompt.contains("first") && en_prompt.contains("then"), "en structuring prompt should list English sequence markers");
    }

    #[test]
    fn test_system_prompt_with_structuring_still_contains_tech_terms() {
        let prompt = build_system_prompt("zh", true, &[], &[]);
        assert!(prompt.contains("React"), "structuring prompt should still contain tech term examples");
        assert!(prompt.contains("瑞嗯特"), "structuring prompt should still contain phonetic examples");
    }

    #[test]
    fn test_system_prompt_with_structuring_allows_markdown_output() {
        let zh_prompt = build_system_prompt("zh", true, &[], &[]);
        assert!(!zh_prompt.contains("不要任何 markdown"), "zh structuring prompt should NOT forbid markdown since lists use markdown formatting");

        let en_prompt = build_system_prompt("en", true, &[], &[]);
        assert!(!en_prompt.contains("no markdown"), "en structuring prompt should NOT forbid markdown since lists use markdown formatting");
    }

    #[test]
    fn test_system_prompt_without_structuring_forbids_markdown() {
        let zh_prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(zh_prompt.contains("不要任何 markdown"), "zh non-structuring prompt should forbid markdown output");

        let en_prompt = build_system_prompt("en", false, &[], &[]);
        assert!(en_prompt.contains("no markdown"), "en non-structuring prompt should forbid markdown output");
    }

    #[tokio::test]
    async fn test_optimize_text_with_structuring_sends_structuring_prompt() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("structured output")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("test text", "zh", &[], true, &[], None, &[]).await;
        assert!(result.is_ok());

        let received = mock_server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();
        let system_content = body["messages"][0]["content"].as_str().unwrap_or("");

        assert!(
            system_content.contains("编号列表"),
            "With text_structuring=true, zh system prompt should mention numbered list output"
        );
    }

    #[tokio::test]
    async fn test_optimize_text_without_structuring_sends_plain_prompt() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("plain output")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();

        let result = client.optimize_text("test text", "zh", &[], false, &[], None, &[]).await;
        assert!(result.is_ok());

        let received = mock_server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();
        let system_content = body["messages"][0]["content"].as_str().unwrap_or("");

         assert!(
            !system_content.contains("编号列表"),
            "With text_structuring=false, zh system prompt should NOT mention numbered list output"
        );
    }

    // --- User Tags Tests ---

    #[test]
    fn test_system_prompt_with_user_tags() {
        let tags = vec!["Developer".to_string(), "Frontend".to_string(), "AI".to_string()];
        let prompt = build_system_prompt("zh", false, &[], &tags);
        assert!(prompt.contains("用户领域"), "zh prompt with user_tags should contain 用户领域 section");
        assert!(prompt.contains("Developer"), "prompt should contain the tag 'Developer'");
        assert!(prompt.contains("Frontend"), "prompt should contain the tag 'Frontend'");
        assert!(prompt.contains("AI"), "prompt should contain the tag 'AI'");
        assert!(prompt.contains("歧义"), "zh prompt should mention domain-specific interpretation");
    }

    #[test]
    fn test_system_prompt_without_user_tags() {
        let zh_prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(!zh_prompt.contains("用户领域"), "zh prompt without user_tags should NOT contain 用户领域 section");
        let en_prompt = build_system_prompt("en", false, &[], &[]);
        assert!(!en_prompt.contains("User Profile"), "en prompt without user_tags should NOT contain User Profile section");
    }

    #[test]
    fn test_system_prompt_user_tags_with_vocabulary() {
        let vocab = vec!["Kubernetes".to_string()];
        let tags = vec!["DevOps".to_string()];
        let zh_prompt = build_system_prompt("zh", false, &vocab, &tags);
        assert!(zh_prompt.contains("自定义词汇"), "zh prompt should contain vocabulary section");
        assert!(zh_prompt.contains("用户领域"), "zh prompt should contain tags section");
        assert!(zh_prompt.contains("Kubernetes"), "zh prompt should contain vocabulary term");
        assert!(zh_prompt.contains("DevOps"), "zh prompt should contain tag");

        let en_prompt = build_system_prompt("en", false, &vocab, &tags);
        assert!(en_prompt.contains("Custom Vocabulary"), "en prompt should contain vocabulary section");
        assert!(en_prompt.contains("User Profile"), "en prompt should contain tags section");
    }

    #[test]
    fn test_system_prompt_user_tags_all_languages() {
        let tags = vec!["Designer".to_string()];

        let zh_prompt = build_system_prompt("zh", false, &[], &tags);
        assert!(zh_prompt.contains("用户领域"), "zh prompt with tags should contain 用户领域 section");
        assert!(zh_prompt.contains("Designer"), "zh prompt should contain the tag");

        for lang in &["en", "auto", "ja"] {
            let prompt = build_system_prompt(lang, false, &[], &tags);
            assert!(prompt.contains("User Profile"), "prompt for '{}' with tags should contain User Profile section", lang);
            assert!(prompt.contains("Designer"), "prompt for '{}' should contain the tag", lang);
        }
    }

    #[tokio::test]
    async fn test_optimize_text_with_user_tags_sends_tags_in_prompt() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(success_response("optimized")),
            )
            .mount(&mock_server)
            .await;

        let client = LlmClient::new("test-key".to_string(), mock_server.uri(), None).unwrap();
        let tags = vec!["Developer".to_string(), "Rust".to_string()];

        let result = client.optimize_text("test text", "zh", &[], false, &[], None, &tags).await;
        assert!(result.is_ok());

        let received = mock_server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();
        let system_content = body["messages"][0]["content"].as_str().unwrap_or("");

        assert!(
            system_content.contains("用户领域"),
            "With user_tags, zh system prompt should contain 用户领域 section"
        );
        assert!(
            system_content.contains("Developer") && system_content.contains("Rust"),
            "System prompt should contain the user tags"
        );
    }
}
