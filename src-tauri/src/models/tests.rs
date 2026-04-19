use super::manager;
use super::registry;

#[test]
fn test_get_model_found() {
    let model = registry::get_model("whisper-base");
    assert!(model.is_some());
    assert_eq!(model.unwrap().id, "whisper-base");
}

#[test]
fn test_get_model_not_found() {
    assert!(registry::get_model("nonexistent").is_none());
}

#[test]
fn test_all_models_have_files() {
    for model in registry::ALL_MODELS {
        assert!(!model.files.is_empty(), "Model {} has no files", model.id);
    }
}

#[test]
fn test_recommended_models_for_chinese() {
    let recs = registry::recommended_models_for_language("zh");
    let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
    assert!(ids.contains(&"sensevoice-small"));
    assert!(ids.contains(&"paraformer-zh"));
}

#[test]
fn test_recommended_models_for_english() {
    let recs = registry::recommended_models_for_language("en");
    let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
    assert!(ids.contains(&"whisper-large-v3-turbo"));
    assert!(ids.contains(&"moonshine-base-en"));
}

#[test]
fn test_recommended_models_fallback() {
    let recs = registry::recommended_models_for_language("unknown-lang");
    assert!(recs.is_empty());
}

#[test]
fn test_suggest_switch_when_needed() {
    let suggestion = registry::suggest_model_switch("whisper-base", "zh");
    assert!(suggestion.is_some());
    let recs = suggestion.unwrap();
    assert!(recs.len() >= 2);
    let ids: Vec<&str> = recs.iter().map(|(id, _, _)| *id).collect();
    assert!(ids.contains(&"sensevoice-small"));
    assert!(ids.contains(&"paraformer-zh"));
}

#[test]
fn test_suggest_switch_not_needed_any_recommended() {
    let suggestion = registry::suggest_model_switch("sensevoice-small", "zh");
    assert!(suggestion.is_none());
    let suggestion2 = registry::suggest_model_switch("paraformer-zh", "zh");
    assert!(suggestion2.is_none());
}

#[test]
fn test_suggest_switch_no_nag_for_unknown_lang() {
    let suggestion = registry::suggest_model_switch("whisper-base", "unknown-lang");
    assert!(suggestion.is_none());
}

#[test]
fn test_list_models_returns_all() {
    let models = manager::list_models_with_status("whisper-base");
    assert_eq!(models.len(), registry::ALL_MODELS.len());
}

#[test]
fn test_list_models_marks_active() {
    let models = manager::list_models_with_status("whisper-base");
    let active_count = models.iter().filter(|m| m.is_active).count();
    assert_eq!(active_count, 1);
    assert!(models.iter().any(|m| m.id == "whisper-base" && m.is_active));
}

#[test]
fn test_paraformer_trilingual_registered() {
    let model = registry::get_model("paraformer-trilingual")
        .expect("paraformer-trilingual must be registered");
    assert_eq!(model.backend, registry::BackendKind::Paraformer);
    assert_eq!(model.files.len(), 2, "should have model + tokens");
    let paths: Vec<&str> = model.files.iter().map(|f| f.relative_path).collect();
    assert!(paths.contains(&"model.int8.onnx"));
    assert!(paths.contains(&"tokens.txt"));
}

#[test]
fn test_paraformer_trilingual_recommended_languages() {
    for lang in ["zh", "en", "yue"] {
        let recs = registry::recommended_models_for_language(lang);
        let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
        assert!(ids.contains(&"paraformer-trilingual"),
            "paraformer-trilingual should be recommended for {}, got: {:?}", lang, ids);
    }
    for lang in ["ja", "ko", "auto"] {
        let recs = registry::recommended_models_for_language(lang);
        let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
        assert!(!ids.contains(&"paraformer-trilingual"),
            "paraformer-trilingual should NOT be recommended for {}, got: {:?}", lang, ids);
    }
}

#[test]
fn test_fire_red_asr_v1_registered() {
    let model = registry::get_model("fire-red-asr-v1")
        .expect("fire-red-asr-v1 must be registered");
    assert_eq!(model.backend, registry::BackendKind::FireRedAsr);
    assert_eq!(model.files.len(), 3, "should have encoder + decoder + tokens");
    let paths: Vec<&str> = model.files.iter().map(|f| f.relative_path).collect();
    assert!(paths.contains(&"encoder.int8.onnx"));
    assert!(paths.contains(&"decoder.int8.onnx"));
    assert!(paths.contains(&"tokens.txt"));
}

#[test]
fn test_fire_red_asr_v1_recommended_for_chinese_only() {
    let recs = registry::recommended_models_for_language("zh");
    let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
    assert!(ids.contains(&"fire-red-asr-v1"),
        "fire-red-asr-v1 should be recommended for zh, got: {:?}", ids);
    for lang in ["en", "ja", "ko", "yue", "auto"] {
        let recs = registry::recommended_models_for_language(lang);
        let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
        assert!(!ids.contains(&"fire-red-asr-v1"),
            "fire-red-asr-v1 should NOT be recommended for {}, got: {:?}", lang, ids);
    }
}

#[test]
fn test_fire_red_asr_model_paths() {
    let (enc, dec, tokens) = manager::fire_red_asr_model_paths("fire-red-asr-v1")
        .expect("paths should resolve");
    assert!(enc.ends_with(std::path::Path::new("encoder.int8.onnx")));
    assert!(dec.ends_with(std::path::Path::new("decoder.int8.onnx")));
    assert!(tokens.ends_with(std::path::Path::new("tokens.txt")));
}

#[test]
fn test_zipformer_ctc_zh_registered() {
    let model = registry::get_model("zipformer-ctc-zh")
        .expect("zipformer-ctc-zh must be registered");
    assert_eq!(model.backend, registry::BackendKind::ZipformerCtc);
    assert_eq!(model.files.len(), 2, "should have model + tokens (bbpe.model not downloaded)");
    let paths: Vec<&str> = model.files.iter().map(|f| f.relative_path).collect();
    assert!(paths.contains(&"model.int8.onnx"));
    assert!(paths.contains(&"tokens.txt"));
}

#[test]
fn test_zipformer_ctc_zh_recommended_for_chinese_only() {
    let recs = registry::recommended_models_for_language("zh");
    let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
    assert!(ids.contains(&"zipformer-ctc-zh"),
        "zipformer-ctc-zh should be recommended for zh, got: {:?}", ids);
    for lang in ["en", "ja", "ko", "yue", "auto"] {
        let recs = registry::recommended_models_for_language(lang);
        let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
        assert!(!ids.contains(&"zipformer-ctc-zh"),
            "zipformer-ctc-zh should NOT be recommended for {}, got: {:?}", lang, ids);
    }
}

#[test]
fn test_zipformer_ctc_model_paths() {
    let (model, tokens) = manager::zipformer_ctc_model_paths("zipformer-ctc-zh")
        .expect("paths should resolve");
    assert!(model.ends_with(std::path::Path::new("model.int8.onnx")));
    assert!(tokens.ends_with(std::path::Path::new("tokens.txt")));
}
