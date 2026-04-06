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
