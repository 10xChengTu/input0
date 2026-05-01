use crate::errors::AppError;
use crate::llm::client::LlmClient;
use crate::history;
use crate::config;

#[tauri::command]
pub async fn optimize_text(
    text: String,
    api_key: String,
    base_url: String,
    language: String,
) -> Result<String, AppError> {
    let client = LlmClient::new(api_key, base_url, None)?;
    let history = history::load_history();
    let config = config::load()?;
    let vocabulary = crate::vocabulary::load_vocabulary();

    let custom_active = crate::llm::client::is_custom_prompt_active(
        config.custom_prompt_enabled,
        &config.custom_prompt,
        &language,
    );
    let clipboard = if custom_active && config.custom_prompt.contains("{{clipboard}}") {
        arboard::Clipboard::new()
            .and_then(|mut cb| cb.get_text())
            .ok()
    } else {
        None
    };

    let opts = crate::llm::client::OptimizeOptions {
        language: &language,
        history: &history,
        text_structuring: config.text_structuring,
        structuring_prompt: &config.structuring_prompt,
        vocabulary: &vocabulary,
        source_app: None,
        user_tags: &config.user_tags,
        custom_prompt_enabled: config.custom_prompt_enabled,
        custom_prompt: &config.custom_prompt,
        clipboard: clipboard.as_deref(),
    };

    client.optimize_text_with_options(&text, &opts).await
}

#[tauri::command]
pub async fn get_default_structuring_module(language: String) -> Result<String, AppError> {
    // Returns the language-appropriate built-in structuring module text, used
    // by the editor to seed the textarea on first open. The user can edit it;
    // when their saved string equals this default (or is empty), the runtime
    // path uses the built-in default verbatim.
    Ok(crate::llm::client::structuring_module_for(&language).to_string())
}

#[tauri::command]
pub async fn test_api_connection(
    api_key: String,
    base_url: String,
    model: String,
) -> Result<String, AppError> {
    let model_opt = if model.is_empty() { None } else { Some(model) };
    let client = LlmClient::new(api_key, base_url, model_opt)?;
    client.test_connection().await
}

#[tauri::command]
pub async fn get_default_prompt_template(language: String) -> Result<String, AppError> {
    // Returns the built-in system prompt as a template, with `{{vocabulary}}`
    // and `{{user_tags}}` placeholders left in so the user can see which parts
    // get substituted at runtime. Single canonical body per language —
    // structuring rules are layered in at runtime via the toggle, not embedded
    // in the editable template.
    Ok(crate::llm::client::build_default_template(&language))
}

