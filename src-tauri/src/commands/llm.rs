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

    let custom_active = config.custom_prompt_enabled && !config.custom_prompt.trim().is_empty();
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
    // Returns the built-in system prompt for a given language so the frontend
    // can use it as the editor's placeholder/default content.
    let config = config::load()?;
    let vocabulary = crate::vocabulary::load_vocabulary();
    Ok(crate::llm::client::build_system_prompt(
        &language,
        config.text_structuring,
        &vocabulary,
        &config.user_tags,
    ))
}

#[tauri::command]
pub async fn preview_custom_prompt(template: String) -> Result<String, AppError> {
    // Renders the user's draft template against the current real context
    // (clipboard, vocabulary, user_tags, history, active_app=None) and appends
    // the safety footer so the user sees exactly what the LLM would receive.
    let config = config::load()?;
    let history = history::load_history();
    let vocabulary = crate::vocabulary::load_vocabulary();

    let clipboard = if template.contains("{{clipboard}}") {
        arboard::Clipboard::new().and_then(|mut cb| cb.get_text()).ok()
    } else {
        None
    };

    let ctx = crate::llm::template::TemplateContext {
        clipboard: clipboard.as_deref(),
        vocabulary: &vocabulary,
        user_tags: &config.user_tags,
        active_app: None,
        language: &config.language,
        history: &history,
    };

    let body = crate::llm::template::render_template(&template, &ctx);
    Ok(format!("{body}\n\n{}", crate::llm::client::safety_footer(&config.language)))
}
