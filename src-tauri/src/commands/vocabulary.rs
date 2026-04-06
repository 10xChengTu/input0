use crate::config;
use crate::errors::AppError;
use crate::llm::client::LlmClient;
use crate::vocabulary;

#[tauri::command]
pub async fn get_vocabulary() -> Result<Vec<String>, AppError> {
    Ok(vocabulary::load_vocabulary())
}

#[tauri::command]
pub async fn set_vocabulary(entries: Vec<String>) -> Result<(), AppError> {
    vocabulary::save_vocabulary(&entries)
}

#[tauri::command]
pub async fn add_vocabulary_entry(term: String) -> Result<bool, AppError> {
    vocabulary::add_entry(term)
}

#[tauri::command]
pub async fn remove_vocabulary_entry(term: String) -> Result<bool, AppError> {
    vocabulary::remove_entry(&term)
}

#[tauri::command]
pub async fn validate_and_add_vocabulary(
    original: String,
    correct: String,
) -> Result<bool, AppError> {
    let config = config::load()?;
    if config.api_key.is_empty() {
        return Err(AppError::Llm(
            "API Key is required to validate vocabulary entries".to_string(),
        ));
    }
    let model = if config.model.is_empty() {
        None
    } else {
        Some(config.model.clone())
    };
    let client = LlmClient::new(config.api_key, config.api_base_url, model)?;
    let is_valid = client.validate_vocabulary(&original, &correct).await?;
    if is_valid {
        vocabulary::add_entry(correct)?;
    }
    Ok(is_valid)
}
