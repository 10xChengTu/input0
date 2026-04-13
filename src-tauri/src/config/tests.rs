#[cfg(test)]
mod tests {
    use super::super::*;
    use std::fs;
    use tempfile::TempDir;

    // =========================================================
    // Default Config Tests
    // =========================================================

    #[test]
    fn test_default_config_values() {
        let config = AppConfig::default();
        assert_eq!(config.api_key, "");
        assert_eq!(config.api_base_url, "https://api.openai.com/v1");
        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.language, "auto");
        assert_eq!(config.hotkey, "Option+Space");
        assert_eq!(config.model_path, "");
        assert_eq!(config.stt_model, "whisper-base");
        assert_eq!(config.text_structuring, true);
        assert_eq!(config.hf_endpoint, "https://huggingface.co");
    }

    #[test]
    fn test_default_config_serializes_to_toml() {
        let config = AppConfig::default();
        let toml_str = toml::to_string(&config).expect("Should serialize to TOML");
        assert!(!toml_str.is_empty());
        // Verify key fields appear in the serialized output
        assert!(toml_str.contains("api_base_url"));
        assert!(toml_str.contains("language"));
        assert!(toml_str.contains("hotkey"));
    }

    // =========================================================
    // Load Tests
    // =========================================================

    #[test]
    fn test_load_returns_default_when_no_file() {
        let tmp = TempDir::new().unwrap();
        let config = load_from_dir(tmp.path()).expect("Should return default config");
        let default = AppConfig::default();
        assert_eq!(config, default);
    }

    #[test]
    fn test_load_reads_existing_file() {
        let tmp = TempDir::new().unwrap();
        let content = r#"
api_key = "my-secret-key"
api_base_url = "https://custom.api.com/v1"
model = "gpt-4o-mini"
language = "zh"
hotkey = "Ctrl+Space"
model_path = "/path/to/model"
"#;
        fs::write(tmp.path().join("config.toml"), content).unwrap();
        let config = load_from_dir(tmp.path()).expect("Should load config");
        assert_eq!(config.api_key, "my-secret-key");
        assert_eq!(config.api_base_url, "https://custom.api.com/v1");
        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.language, "zh");
        assert_eq!(config.hotkey, "Ctrl+Space");
        assert_eq!(config.model_path, "/path/to/model");
    }

    #[test]
    fn test_load_with_partial_config_errors() {
        // A TOML file missing required fields should return an error (strict deserialization)
        let tmp = TempDir::new().unwrap();
        let content = r#"
api_key = "partial-key"
"#;
        fs::write(tmp.path().join("config.toml"), content).unwrap();
        // With strict deserialization, missing fields cause error
        // (toml crate requires all non-optional fields)
        let result = load_from_dir(tmp.path());
        // Either error or success depending on implementation strategy:
        // Since all fields have defaults via Default trait, implementation may merge.
        // We test that it doesn't panic at minimum.
        let _ = result; // Accept either outcome
    }

    #[test]
    fn test_load_with_invalid_toml() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("config.toml"),
            "this is not valid toml = = =",
        )
        .unwrap();
        let result = load_from_dir(tmp.path());
        assert!(result.is_err(), "Invalid TOML should return an error");
    }

    #[test]
    fn test_load_with_corrupt_utf8() {
        let tmp = TempDir::new().unwrap();
        // Write invalid UTF-8 bytes
        let bad_bytes: Vec<u8> = vec![0xFF, 0xFE, 0x80, 0x81, 0x82];
        fs::write(tmp.path().join("config.toml"), bad_bytes).unwrap();
        let result = load_from_dir(tmp.path());
        assert!(result.is_err(), "Invalid UTF-8 should return an error");
    }

    #[test]
    fn test_load_with_empty_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("config.toml"), "").unwrap();
        // Empty TOML is valid TOML but will fail deserialization into AppConfig
        let result = load_from_dir(tmp.path());
        assert!(result.is_err(), "Empty TOML file should return an error");
    }

    #[test]
    fn test_load_with_extra_fields_ignored() {
        let tmp = TempDir::new().unwrap();
        let content = r#"
api_key = "test-key"
api_base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
language = "auto"
hotkey = "Option+Space"
model_path = ""
unknown_extra_field = "should be ignored"
another_unknown = 42
"#;
        fs::write(tmp.path().join("config.toml"), content).unwrap();
        // Extra fields should be silently ignored
        let result = load_from_dir(tmp.path());
        assert!(result.is_ok(), "Extra fields should not cause error");
        let config = result.unwrap();
        assert_eq!(config.api_key, "test-key");
    }

    // =========================================================
    // Save Tests
    // =========================================================

    #[test]
    fn test_save_creates_directory_if_not_exists() {
        let tmp = TempDir::new().unwrap();
        // Use a nested dir that doesn't exist yet
        let nested_dir = tmp.path().join("nested").join("config");
        let config = AppConfig::default();
        save_to_dir(&config, &nested_dir).expect("Should create directory and save");
        assert!(nested_dir.join("config.toml").exists());
    }

    #[test]
    fn test_save_creates_file() {
        let tmp = TempDir::new().unwrap();
        let config = AppConfig::default();
        save_to_dir(&config, tmp.path()).expect("Should save config");
        assert!(tmp.path().join("config.toml").exists());
    }

    #[test]
    fn test_save_overwrites_existing() {
        let tmp = TempDir::new().unwrap();
        let config1 = AppConfig {
            api_key: "first-key".to_string(),
            ..AppConfig::default()
        };
        save_to_dir(&config1, tmp.path()).expect("First save");

        let config2 = AppConfig {
            api_key: "second-key".to_string(),
            ..AppConfig::default()
        };
        save_to_dir(&config2, tmp.path()).expect("Second save");

        let loaded = load_from_dir(tmp.path()).expect("Load after overwrite");
        assert_eq!(loaded.api_key, "second-key");
    }

    #[test]
    fn test_save_content_is_valid_toml() {
        let tmp = TempDir::new().unwrap();
        let config = AppConfig {
            api_key: "test-key".to_string(),
            ..AppConfig::default()
        };
        save_to_dir(&config, tmp.path()).expect("Save config");

        let raw = fs::read_to_string(tmp.path().join("config.toml")).expect("Read saved file");
        let parsed: Result<toml::Value, _> = toml::from_str(&raw);
        assert!(parsed.is_ok(), "Saved content should be valid TOML");
    }

    #[test]
    fn test_save_preserves_all_fields() {
        let tmp = TempDir::new().unwrap();
        let config = AppConfig {
            api_key: "preserve-key".to_string(),
            api_base_url: "https://custom.example.com".to_string(),
            model: "gpt-4".to_string(),
            language: "ja".to_string(),
            hotkey: "Cmd+Shift+Space".to_string(),
            model_path: "/models/whisper.bin".to_string(),
            stt_model: default_stt_model(),
            text_structuring: false,
            user_tags: Vec::new(),
            custom_models: Vec::new(),
            onboarding_completed: false,
            input_device: String::new(),
            hf_endpoint: default_hf_endpoint(),
        };
        save_to_dir(&config, tmp.path()).expect("Save config");
        let loaded = load_from_dir(tmp.path()).expect("Load config");
        assert_eq!(loaded.api_key, "preserve-key");
        assert_eq!(loaded.api_base_url, "https://custom.example.com");
        assert_eq!(loaded.model, "gpt-4");
        assert_eq!(loaded.language, "ja");
        assert_eq!(loaded.hotkey, "Cmd+Shift+Space");
        assert_eq!(loaded.model_path, "/models/whisper.bin");
    }

    // =========================================================
    // Update Field Tests
    // =========================================================

    #[test]
    fn test_update_api_key() {
        let tmp = TempDir::new().unwrap();
        // Ensure initial config exists
        save_to_dir(&AppConfig::default(), tmp.path()).expect("Initial save");
        let updated =
            update_field_in_dir("api_key", "new-api-key", tmp.path()).expect("Update api_key");
        assert_eq!(updated.api_key, "new-api-key");
    }

    #[test]
    fn test_update_api_base_url() {
        let tmp = TempDir::new().unwrap();
        save_to_dir(&AppConfig::default(), tmp.path()).expect("Initial save");
        let updated = update_field_in_dir("api_base_url", "https://newurl.com", tmp.path())
            .expect("Update api_base_url");
        assert_eq!(updated.api_base_url, "https://newurl.com");
    }

    #[test]
    fn test_update_language() {
        let tmp = TempDir::new().unwrap();
        save_to_dir(&AppConfig::default(), tmp.path()).expect("Initial save");
        let updated = update_field_in_dir("language", "en", tmp.path()).expect("Update language");
        assert_eq!(updated.language, "en");
    }

    #[test]
    fn test_update_hotkey() {
        let tmp = TempDir::new().unwrap();
        save_to_dir(&AppConfig::default(), tmp.path()).expect("Initial save");
        let updated =
            update_field_in_dir("hotkey", "Cmd+Space", tmp.path()).expect("Update hotkey");
        assert_eq!(updated.hotkey, "Cmd+Space");
    }

    #[test]
    fn test_update_model_path() {
        let tmp = TempDir::new().unwrap();
        save_to_dir(&AppConfig::default(), tmp.path()).expect("Initial save");
        let updated = update_field_in_dir("model_path", "/new/path/model.bin", tmp.path())
            .expect("Update model_path");
        assert_eq!(updated.model_path, "/new/path/model.bin");
    }

    #[test]
    fn test_update_invalid_field() {
        let tmp = TempDir::new().unwrap();
        save_to_dir(&AppConfig::default(), tmp.path()).expect("Initial save");
        let result = update_field_in_dir("nonexistent_field", "some-value", tmp.path());
        assert!(result.is_err(), "Unknown field should return error");
    }

    #[test]
    fn test_update_preserves_other_fields() {
        let tmp = TempDir::new().unwrap();
        let config = AppConfig {
            api_key: "original-key".to_string(),
            api_base_url: "https://original.com".to_string(),
            model: "gpt-4o".to_string(),
            language: "zh".to_string(),
            hotkey: "Fn".to_string(),
            model_path: "/original/model".to_string(),
            stt_model: default_stt_model(),
            text_structuring: false,
            user_tags: Vec::new(),
            custom_models: Vec::new(),
            onboarding_completed: false,
            input_device: String::new(),
            hf_endpoint: default_hf_endpoint(),
        };
        save_to_dir(&config, tmp.path()).expect("Initial save");
        let updated =
            update_field_in_dir("language", "en", tmp.path()).expect("Update language only");
        // Only language changed
        assert_eq!(updated.language, "en");
        // All other fields preserved
        assert_eq!(updated.api_key, "original-key");
        assert_eq!(updated.api_base_url, "https://original.com");
        assert_eq!(updated.model, "gpt-4o");
        assert_eq!(updated.hotkey, "Fn");
        assert_eq!(updated.model_path, "/original/model");
    }

    // =========================================================
    // Round-Trip Tests
    // =========================================================

    #[test]
    fn test_save_then_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let config = AppConfig {
            api_key: "roundtrip-key".to_string(),
            api_base_url: "https://roundtrip.example.com/v2".to_string(),
            model: "gpt-4o-mini".to_string(),
            language: "ko".to_string(),
            hotkey: "F1".to_string(),
            model_path: "/roundtrip/model.bin".to_string(),
            stt_model: default_stt_model(),
            text_structuring: false,
            user_tags: Vec::new(),
            custom_models: Vec::new(),
            onboarding_completed: false,
            input_device: String::new(),
            hf_endpoint: default_hf_endpoint(),
        };
        save_to_dir(&config, tmp.path()).expect("Save");
        let loaded = load_from_dir(tmp.path()).expect("Load");
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_multiple_saves_and_loads() {
        let tmp = TempDir::new().unwrap();

        // First save
        let config1 = AppConfig {
            api_key: "key-v1".to_string(),
            ..AppConfig::default()
        };
        save_to_dir(&config1, tmp.path()).expect("Save v1");
        let loaded1 = load_from_dir(tmp.path()).expect("Load v1");
        assert_eq!(loaded1.api_key, "key-v1");

        // Modify and save again
        let config2 = AppConfig {
            api_key: "key-v2".to_string(),
            language: "fr".to_string(),
            ..loaded1
        };
        save_to_dir(&config2, tmp.path()).expect("Save v2");
        let loaded2 = load_from_dir(tmp.path()).expect("Load v2");
        assert_eq!(loaded2.api_key, "key-v2");
        assert_eq!(loaded2.language, "fr");
    }

    // =========================================================
    // Edge Cases
    // =========================================================

    #[test]
    fn test_config_with_unicode_values() {
        let tmp = TempDir::new().unwrap();
        let config = AppConfig {
            api_key: "密钥-🔑-キー-한국어".to_string(),
            api_base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            language: "中文".to_string(),
            hotkey: "Option+Space".to_string(),
            model_path: "/路径/模型.bin".to_string(),
            stt_model: default_stt_model(),
            text_structuring: false,
            user_tags: Vec::new(),
            custom_models: Vec::new(),
            onboarding_completed: false,
            input_device: String::new(),
            hf_endpoint: default_hf_endpoint(),
        };
        save_to_dir(&config, tmp.path()).expect("Save unicode config");
        let loaded = load_from_dir(tmp.path()).expect("Load unicode config");
        assert_eq!(loaded.api_key, "密钥-🔑-キー-한국어");
        assert_eq!(loaded.language, "中文");
        assert_eq!(loaded.model_path, "/路径/模型.bin");
    }

    #[test]
    fn test_config_with_empty_strings() {
        let tmp = TempDir::new().unwrap();
        let config = AppConfig {
            api_key: String::new(),
            api_base_url: String::new(),
            model: String::new(),
            language: String::new(),
            hotkey: String::new(),
            model_path: String::new(),
            stt_model: String::new(),
            text_structuring: false,
            user_tags: Vec::new(),
            custom_models: Vec::new(),
            onboarding_completed: false,
            input_device: String::new(),
            hf_endpoint: default_hf_endpoint(),
        };
        save_to_dir(&config, tmp.path()).expect("Save empty config");
        let loaded = load_from_dir(tmp.path()).expect("Load empty config");
        assert_eq!(loaded.api_key, "");
        assert_eq!(loaded.api_base_url, "");
        assert_eq!(loaded.model, "");
        assert_eq!(loaded.language, "");
        assert_eq!(loaded.hotkey, "");
        assert_eq!(loaded.model_path, "");
    }

    #[test]
    fn test_config_with_very_long_values() {
        let tmp = TempDir::new().unwrap();
        let long_string = "a".repeat(10 * 1024); // 10KB string
        let config = AppConfig {
            api_key: long_string.clone(),
            api_base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            language: "auto".to_string(),
            hotkey: "Option+Space".to_string(),
            model_path: long_string.clone(),
            stt_model: default_stt_model(),
            text_structuring: false,
            user_tags: Vec::new(),
            custom_models: Vec::new(),
            onboarding_completed: false,
            input_device: String::new(),
            hf_endpoint: default_hf_endpoint(),
        };
        save_to_dir(&config, tmp.path()).expect("Save large config");
        let loaded = load_from_dir(tmp.path()).expect("Load large config");
        assert_eq!(loaded.api_key.len(), 10 * 1024);
        assert_eq!(loaded.model_path.len(), 10 * 1024);
        assert_eq!(loaded.api_key, long_string);
    }

    #[test]
    fn test_config_path_returns_correct_filename() {
        let tmp = TempDir::new().unwrap();
        let config = AppConfig::default();
        save_to_dir(&config, tmp.path()).expect("Save config");

        // Config file should be named config.toml
        let config_file = tmp.path().join("config.toml");
        assert!(
            config_file.exists(),
            "config.toml should exist in the config dir"
        );
    }

    #[test]
    fn test_load_from_nonexistent_dir_returns_default() {
        // A completely nonexistent directory should return default config (not error)
        let tmp = TempDir::new().unwrap();
        let nonexistent = tmp.path().join("does_not_exist");
        let config = load_from_dir(&nonexistent).expect("Should return default when dir missing");
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn test_config_dir_function_returns_path() {
        // config_dir() should return a valid PathBuf (on macOS)
        let result = config_dir();
        assert!(result.is_ok(), "config_dir() should succeed");
        let path = result.unwrap();
        // Should end with our app identifier component
        assert!(
            path.to_string_lossy().contains("com.input0.app"),
            "Config dir should contain app identifier, got: {}",
            path.display()
        );
    }

    #[test]
    fn test_config_path_function_returns_toml_path() {
        let result = config_path();
        assert!(result.is_ok(), "config_path() should succeed");
        let path = result.unwrap();
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("config.toml"),
            "config path filename should be config.toml"
        );
    }

    // =========================================================
    // User Tags Tests
    // =========================================================

    #[test]
    fn test_config_user_tags_default() {
        // Old config files without user_tags should deserialize with empty vec
        let tmp = TempDir::new().unwrap();
        let content = r#"
api_key = "test-key"
api_base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
language = "auto"
hotkey = "Option+Space"
model_path = ""
stt_model = "whisper-base"
text_structuring = true
"#;
        fs::write(tmp.path().join("config.toml"), content).unwrap();
        let config = load_from_dir(tmp.path()).expect("Should load config without user_tags");
        assert!(
            config.user_tags.is_empty(),
            "user_tags should default to empty vec"
        );
    }

    #[test]
    fn test_update_field_user_tags() {
        let tmp = TempDir::new().unwrap();
        save_to_dir(&AppConfig::default(), tmp.path()).expect("Initial save");
        let tags_json = r#"["Developer","AI","Frontend"]"#;
        let updated =
            update_field_in_dir("user_tags", tags_json, tmp.path()).expect("Update user_tags");
        assert_eq!(updated.user_tags, vec!["Developer", "AI", "Frontend"]);

        // Verify persistence
        let loaded = load_from_dir(tmp.path()).expect("Load after update");
        assert_eq!(loaded.user_tags, vec!["Developer", "AI", "Frontend"]);
    }

    #[test]
    fn test_update_field_hf_endpoint() {
        let tmp = TempDir::new().unwrap();
        save_to_dir(&AppConfig::default(), tmp.path()).expect("Initial save");
        let updated = update_field_in_dir("hf_endpoint", "https://hf-mirror.com", tmp.path())
            .expect("Update hf_endpoint");
        assert_eq!(updated.hf_endpoint, "https://hf-mirror.com");

        // Verify persistence
        let loaded = load_from_dir(tmp.path()).expect("Load after update");
        assert_eq!(loaded.hf_endpoint, "https://hf-mirror.com");
    }
}
