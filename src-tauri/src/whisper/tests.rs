#[cfg(test)]
mod tests {
    use crate::whisper::transcriber;

    #[test]
    fn test_is_model_loaded_initially_false() {
        assert!(!transcriber::is_model_loaded());
    }

    #[test]
    fn test_init_model_nonexistent_file() {
        let result = transcriber::init_model(std::path::Path::new("/nonexistent/path/model.bin"));
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("not found") || err_str.contains("already initialized"));
    }

    #[test]
    fn test_transcribe_without_model() {
        if transcriber::is_model_loaded() {
            return;
        }
        let result = transcriber::transcribe(&[0.0f32; 100], "en");
        assert!(result.is_err());
    }

    #[test]
    fn test_transcribe_empty_audio_without_model() {
        if transcriber::is_model_loaded() {
            return;
        }
        let result = transcriber::transcribe(&[], "en");
        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[test]
    fn test_initial_prompt_zh_cn_returns_simplified_prompt() {
        let prompt = transcriber::initial_prompt_for_language("zh-CN");
        assert_eq!(prompt, Some("以下是普通话的句子。"));
    }

    #[test]
    fn test_initial_prompt_zh_tw_returns_traditional_prompt() {
        let prompt = transcriber::initial_prompt_for_language("zh-TW");
        assert_eq!(prompt, Some("以下是國語的句子。"));
    }

    #[test]
    fn test_initial_prompt_legacy_zh_still_returns_simplified() {
        // Defensive: callers should never pass the legacy "zh" code post-migration,
        // but the helper stays tolerant so a stale call site can't regress to None.
        let prompt = transcriber::initial_prompt_for_language("zh");
        assert_eq!(prompt, Some("以下是普通话的句子。"));
    }

    #[test]
    fn test_initial_prompt_other_languages_returns_none() {
        assert!(transcriber::initial_prompt_for_language("en").is_none());
        assert!(transcriber::initial_prompt_for_language("ja").is_none());
        assert!(transcriber::initial_prompt_for_language("ko").is_none());
        assert!(transcriber::initial_prompt_for_language("auto").is_none());
    }

    #[test]
    #[ignore]
    fn test_init_and_transcribe() {
        let model_path = std::path::Path::new("/tmp/ggml-base.bin");
        if !model_path.exists() {
            eprintln!("Skipping: model file not found at {:?}", model_path);
            return;
        }

        let init_result = transcriber::init_model(model_path);
        assert!(
            init_result.is_ok()
                || init_result
                    .unwrap_err()
                    .to_string()
                    .contains("already initialized")
        );

        let silence = vec![0.0f32; 16000];
        let result = transcriber::transcribe(&silence, "en");
        assert!(result.is_ok(), "Transcription of silence should succeed");
    }
}
