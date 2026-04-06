use tauri::command;

use crate::config::{self, AppConfig};
use crate::errors::AppError;

#[command]
pub async fn get_config() -> Result<AppConfig, AppError> {
    config::load()
}

#[command]
pub async fn save_config(config: AppConfig) -> Result<(), AppError> {
    config::save(&config)
}

#[command]
pub async fn update_config_field(field: String, value: String) -> Result<AppConfig, AppError> {
    config::update_field(&field, &value)
}
