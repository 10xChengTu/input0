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
        assert!(prompt.contains("Tauri"), "zh prompt should contain project-specific term Tauri");
        assert!(prompt.contains("Vite"), "zh prompt should contain project-specific term Vite");
    }

    #[test]
    fn test_system_prompt_en_includes_phonetic_table() {
        let prompt = build_system_prompt("en", false, &[], &[]);
        assert!(prompt.contains("瑞嗯特"), "en prompt should contain Chinese phonetic examples for code-switching");
        assert!(prompt.contains("React"), "en prompt should contain React");
        assert!(prompt.contains("filler"), "en prompt should mention filler words");
        assert!(prompt.contains("grammar"), "en prompt should mention grammar");
        assert!(prompt.contains("English"), "en prompt should mention English");
        assert!(prompt.contains("code-switches"), "en prompt should mention code-switching");
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
        for lang in &["zh", "en", "auto", "ja"] {
            let prompt = build_system_prompt(lang, false, &[], &[]);
            assert!(prompt.contains("filler"), "prompt for '{}' should mention filler words", lang);
            assert!(prompt.contains("grammar"), "prompt for '{}' should mention grammar", lang);
            assert!(prompt.contains("ONLY the corrected text"), "prompt for '{}' should have output-only instruction", lang);
        }
    }

    #[test]
    fn test_system_prompt_zh_preserves_variant() {
        let prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(
            prompt.contains("Preserve the Chinese variant"),
            "zh prompt should preserve speaker's Chinese variant"
        );
        assert!(
            !prompt.contains("简体中文"),
            "zh prompt must NOT force simplified Chinese"
        );
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
    fn test_system_prompt_contains_active_app_instructions() {
        let prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(prompt.contains("active application"), "System prompt should mention active application usage");
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
        assert_eq!(messages[1]["content"], "hello um world");
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
        assert_eq!(messages[2]["content"], "hello world");
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
            system_content.to_lowercase().contains("grammar"),
            "System prompt should mention grammar, got: {}",
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
        assert_eq!(
            last_msg["content"].as_str().unwrap(), raw,
            "Last user message content should be exactly the raw text"
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
        for lang in &["zh", "en", "auto"] {
            let prompt = build_system_prompt(lang, false, &[], &[]);
            assert!(!prompt.contains("Numbered & Bulleted Lists"), "prompt for '{}' without structuring should NOT contain list instructions", lang);
            assert!(!prompt.contains("Text Structuring"), "prompt for '{}' without structuring should NOT contain structuring section", lang);
        }
    }

    #[test]
    fn test_system_prompt_with_structuring_contains_list_instructions() {
        for lang in &["zh", "en", "auto"] {
            let prompt = build_system_prompt(lang, true, &[], &[]);
            assert!(prompt.contains("Numbered & Bulleted Lists"), "prompt for '{}' with structuring should contain list instructions", lang);
            assert!(prompt.contains("Text Structuring"), "prompt for '{}' with structuring should contain structuring section", lang);
            assert!(prompt.contains("Paragraph & Line Breaks"), "prompt for '{}' with structuring should contain paragraph instructions", lang);
            assert!(prompt.contains("Punctuation & Symbols"), "prompt for '{}' with structuring should contain punctuation instructions", lang);
            assert!(prompt.contains("Spacing"), "prompt for '{}' with structuring should contain spacing instructions", lang);
        }
    }

    #[test]
    fn test_system_prompt_with_structuring_contains_few_shot_examples() {
        let prompt = build_system_prompt("zh", true, &[], &[]);
        assert!(prompt.contains("1. 把 API 接口设计好"), "structuring prompt should contain numbered list few-shot example");
        assert!(prompt.contains("CI/CD"), "structuring prompt should contain CI/CD in few-shot example");
        assert!(prompt.contains("1. 把游戏打好"), "structuring prompt should contain punctuated enumeration few-shot example");
        assert!(prompt.contains("1. 保证代码质量"), "structuring prompt should contain direct numbered few-shot example");
    }

    #[test]
    fn test_system_prompt_with_structuring_still_contains_tech_terms() {
        let prompt = build_system_prompt("zh", true, &[], &[]);
        assert!(prompt.contains("React"), "structuring prompt should still contain tech term table");
        assert!(prompt.contains("瑞嗯特"), "structuring prompt should still contain phonetic examples");
    }

    #[test]
    fn test_system_prompt_with_structuring_allows_markdown_output() {
        let prompt = build_system_prompt("zh", true, &[], &[]);
        assert!(!prompt.contains("no markdown"), "structuring prompt should NOT forbid markdown since lists use markdown formatting");
    }

    #[test]
    fn test_system_prompt_with_structuring_allows_format_restructuring() {
        let prompt = build_system_prompt("zh", true, &[], &[]);
        assert!(prompt.contains("MAY restructure"), "structuring prompt should allow format restructuring");
        assert!(!prompt.contains("do NOT add, remove, or rewrite"), "structuring prompt should NOT have strict no-rewrite constraint");
    }

    #[test]
    fn test_system_prompt_without_structuring_forbids_markdown() {
        let prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(prompt.contains("no markdown"), "non-structuring prompt should forbid markdown output");
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
            system_content.contains("Text Structuring"),
            "With text_structuring=true, system prompt should contain structuring instructions"
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
            !system_content.contains("Text Structuring"),
            "With text_structuring=false, system prompt should NOT contain structuring instructions"
        );
    }

    // --- User Tags Tests ---

    #[test]
    fn test_system_prompt_with_user_tags() {
        let tags = vec!["Developer".to_string(), "Frontend".to_string(), "AI".to_string()];
        let prompt = build_system_prompt("zh", false, &[], &tags);
        assert!(prompt.contains("User Profile Tags"), "prompt with user_tags should contain User Profile Tags section");
        assert!(prompt.contains("Developer"), "prompt should contain the tag 'Developer'");
        assert!(prompt.contains("Frontend"), "prompt should contain the tag 'Frontend'");
        assert!(prompt.contains("AI"), "prompt should contain the tag 'AI'");
        assert!(prompt.contains("domain-specific"), "prompt should mention domain-specific interpretation");
    }

    #[test]
    fn test_system_prompt_without_user_tags() {
        let prompt = build_system_prompt("zh", false, &[], &[]);
        assert!(!prompt.contains("User Profile Tags"), "prompt without user_tags should NOT contain User Profile Tags section");
    }

    #[test]
    fn test_system_prompt_user_tags_with_vocabulary() {
        let vocab = vec!["Kubernetes".to_string()];
        let tags = vec!["DevOps".to_string()];
        let prompt = build_system_prompt("zh", false, &vocab, &tags);
        assert!(prompt.contains("User Custom Vocabulary"), "should contain vocabulary section");
        assert!(prompt.contains("User Profile Tags"), "should contain tags section");
        assert!(prompt.contains("Kubernetes"), "should contain vocabulary term");
        assert!(prompt.contains("DevOps"), "should contain tag");
    }

    #[test]
    fn test_system_prompt_user_tags_all_languages() {
        let tags = vec!["Designer".to_string()];
        for lang in &["zh", "en", "auto", "ja"] {
            let prompt = build_system_prompt(lang, false, &[], &tags);
            assert!(prompt.contains("User Profile Tags"), "prompt for '{}' with tags should contain User Profile Tags section", lang);
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
            system_content.contains("User Profile Tags"),
            "With user_tags, system prompt should contain User Profile Tags section"
        );
        assert!(
            system_content.contains("Developer") && system_content.contains("Rust"),
            "System prompt should contain the user tags"
        );
    }
}
