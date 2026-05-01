use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::AppError;

#[cfg(test)]
mod tests;

const APP_CONFIG_DIR: &str = "com.input0.app";
const CONFIG_FILENAME: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub api_key: String,
    pub api_base_url: String,
    pub model: String,
    pub language: String,
    pub hotkey: String,
    pub model_path: String,
    #[serde(default = "default_stt_model")]
    pub stt_model: String,
    #[serde(default = "default_text_structuring")]
    pub text_structuring: bool,
    #[serde(default)]
    pub user_tags: Vec<String>,
    #[serde(default)]
    pub custom_models: Vec<String>,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default)]
    pub input_device: String,
    #[serde(default = "default_hf_endpoint")]
    pub hf_endpoint: String,
    #[serde(default)]
    pub custom_prompt_enabled: bool,
    #[serde(default)]
    pub custom_prompt: String,
    #[serde(default)]
    pub structuring_prompt: String,
}

fn default_hf_endpoint() -> String {
    "https://huggingface.co".to_string()
}

fn default_stt_model() -> String {
    "whisper-base".to_string()
}

fn default_text_structuring() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            language: "auto".to_string(),
            hotkey: "Option+Space".to_string(),
            model_path: String::new(),
            stt_model: default_stt_model(),
            text_structuring: true,
            user_tags: Vec::new(),
            custom_models: Vec::new(),
            onboarding_completed: false,
            input_device: String::new(),
            hf_endpoint: default_hf_endpoint(),
            custom_prompt_enabled: false,
            custom_prompt: String::new(),
            structuring_prompt: String::new(),
        }
    }
}

pub fn config_dir() -> Result<PathBuf, AppError> {
    let base = dirs::config_dir().ok_or_else(|| {
        AppError::Config("Could not determine system config directory".to_string())
    })?;
    Ok(base.join(APP_CONFIG_DIR))
}

pub fn config_path() -> Result<PathBuf, AppError> {
    Ok(config_dir()?.join(CONFIG_FILENAME))
}

pub fn load() -> Result<AppConfig, AppError> {
    load_from_dir(&config_dir()?)
}

pub fn save(config: &AppConfig) -> Result<(), AppError> {
    save_to_dir(config, &config_dir()?)
}

pub fn update_field(field: &str, value: &str) -> Result<AppConfig, AppError> {
    update_field_in_dir(field, value, &config_dir()?)
}

pub(crate) fn load_from_dir(dir: &Path) -> Result<AppConfig, AppError> {
    let path = dir.join(CONFIG_FILENAME);
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| AppError::Config(format!("Failed to read config file: {}", e)))?;
    let mut config: AppConfig = toml::from_str(&contents)
        .map_err(|e| AppError::Config(format!("Failed to parse config file: {}", e)))?;

    // One-time migration: an upgraded user may have `custom_prompt` saved as
    // a verbatim pre-v2 default template (which the new prompt builder no
    // longer produces). Treating it as a real custom prompt would silently
    // pin them on stale rules. Clear it so the editor re-shows the current
    // default and the runtime path collapses to built-in.
    if !config.custom_prompt.is_empty()
        && crate::llm::client::is_legacy_default_template(&config.custom_prompt)
    {
        log::info!(
            "config migration: clearing custom_prompt that matched a legacy default template"
        );
        config.custom_prompt = String::new();
        // Persist the cleanup so the on-disk state matches in-memory state.
        // Best-effort: a write failure here only means we'll re-migrate next
        // launch, which is harmless.
        if let Err(e) = save_to_dir(&config, dir) {
            log::warn!("config migration: failed to persist cleaned custom_prompt: {}", e);
        }
    }

    Ok(config)
}

pub(crate) fn save_to_dir(config: &AppConfig, dir: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(dir)
        .map_err(|e| AppError::Config(format!("Failed to create config directory: {}", e)))?;
    let contents = toml::to_string(config)
        .map_err(|e| AppError::Config(format!("Failed to serialize config: {}", e)))?;
    std::fs::write(dir.join(CONFIG_FILENAME), contents)
        .map_err(|e| AppError::Config(format!("Failed to write config file: {}", e)))?;
    Ok(())
}

pub(crate) fn update_field_in_dir(
    field: &str,
    value: &str,
    dir: &Path,
) -> Result<AppConfig, AppError> {
    let mut config = load_from_dir(dir)?;
    match field {
        "api_key" => config.api_key = value.to_string(),
        "api_base_url" => config.api_base_url = value.to_string(),
        "model" => config.model = value.to_string(),
        "language" => config.language = value.to_string(),
        "hotkey" => config.hotkey = value.to_string(),
        "model_path" => config.model_path = value.to_string(),
        "stt_model" => config.stt_model = value.to_string(),
        "text_structuring" => {
            config.text_structuring = value.eq_ignore_ascii_case("true");
        }
        "user_tags" => {
            config.user_tags = serde_json::from_str(value)
                .map_err(|e| AppError::Config(format!("Invalid JSON for user_tags: {}", e)))?;
        }
        "custom_models" => {
            config.custom_models = serde_json::from_str(value)
                .map_err(|e| AppError::Config(format!("Invalid JSON for custom_models: {}", e)))?;
        }
        "onboarding_completed" => {
            config.onboarding_completed = value.eq_ignore_ascii_case("true");
        }
        "input_device" => config.input_device = value.to_string(),
        "hf_endpoint" => config.hf_endpoint = value.to_string(),
        "custom_prompt_enabled" => {
            config.custom_prompt_enabled = value.eq_ignore_ascii_case("true");
        }
        "custom_prompt" => config.custom_prompt = value.to_string(),
        "structuring_prompt" => config.structuring_prompt = value.to_string(),
        other => {
            return Err(AppError::Config(format!(
                "Unknown config field: '{}'",
                other
            )));
        }
    }
    save_to_dir(&config, dir)?;
    Ok(config)
}
