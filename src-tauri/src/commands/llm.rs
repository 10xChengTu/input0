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
    let text_structuring = config.text_structuring;
    let vocabulary = crate::vocabulary::load_vocabulary();
    client.optimize_text(&text, &language, &history, text_structuring, &vocabulary, None, &config.user_tags).await
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
