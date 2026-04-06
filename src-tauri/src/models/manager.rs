use std::path::PathBuf;

use serde::Serialize;

use crate::config;
use crate::errors::AppError;
use crate::models::registry::{self, ModelInfo, ModelInfoDto};

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub file_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub file_index: usize,
    pub total_files: usize,
}

fn models_dir() -> Result<PathBuf, AppError> {
    let dir = config::config_dir()?.join("models");
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Config(format!("Failed to create models directory: {}", e)))?;
    Ok(dir)
}

fn model_dir(model_id: &str) -> Result<PathBuf, AppError> {
    let dir = models_dir()?.join(model_id);
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::Config(format!("Failed to create model directory: {}", e)))?;
    Ok(dir)
}

pub fn model_file_path(model_id: &str, relative_path: &str) -> Result<PathBuf, AppError> {
    Ok(model_dir(model_id)?.join(relative_path))
}

pub fn is_model_downloaded(model_id: &str) -> bool {
    let info = match registry::get_model(model_id) {
        Some(info) => info,
        None => return false,
    };
    let dir = match models_dir() {
        Ok(d) => d,
        Err(_) => return false,
    };

    info.files.iter().all(|f| {
        let path = dir.join(model_id).join(f.relative_path);
        path.exists() && path.metadata().map(|m| m.len() > 0).unwrap_or(false)
    })
}

pub fn list_models_with_status(active_model_id: &str) -> Vec<ModelInfoDto> {
    registry::ALL_MODELS
        .iter()
        .map(|m| ModelInfoDto {
            id: m.id.to_string(),
            display_name: m.display_name.to_string(),
            description: m.description.to_string(),
            backend: m.backend,
            total_size_bytes: m.total_size_bytes,
            size_display: m.size_display.to_string(),
            best_for_languages: m.best_for_languages.iter().map(|s| s.to_string()).collect(),
            is_downloaded: is_model_downloaded(m.id),
            is_active: m.id == active_model_id,
        })
        .collect()
}

/// Get the primary model file path for a Whisper model (the .bin file).
pub fn whisper_model_path(model_id: &str) -> Result<PathBuf, AppError> {
    let info = registry::get_model(model_id)
        .ok_or_else(|| AppError::Config(format!("Unknown model: {}", model_id)))?;
    let first = info
        .files
        .first()
        .ok_or_else(|| AppError::Config(format!("Model {} has no files", model_id)))?;
    model_file_path(model_id, first.relative_path)
}

pub fn sensevoice_model_paths(model_id: &str) -> Result<(PathBuf, PathBuf), AppError> {
    let dir = model_dir(model_id)?;
    let onnx = dir.join("model.int8.onnx");
    let tokens = dir.join("tokens.txt");
    Ok((onnx, tokens))
}

pub fn paraformer_model_paths(model_id: &str) -> Result<(PathBuf, PathBuf), AppError> {
    let dir = model_dir(model_id)?;
    let onnx = dir.join("model.int8.onnx");
    let tokens = dir.join("tokens.txt");
    Ok((onnx, tokens))
}

pub fn moonshine_model_paths(model_id: &str) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf, PathBuf), AppError> {
    let dir = model_dir(model_id)?;
    let preprocessor = dir.join("preprocess.onnx");
    let encoder = dir.join("encode.int8.onnx");
    let uncached_decoder = dir.join("uncached_decode.int8.onnx");
    let cached_decoder = dir.join("cached_decode.int8.onnx");
    let tokens = dir.join("tokens.txt");
    Ok((preprocessor, encoder, uncached_decoder, cached_decoder, tokens))
}

pub async fn download_model(
    model_id: &str,
    progress_callback: impl Fn(DownloadProgress) + Send + 'static,
) -> Result<(), AppError> {
    let info: &ModelInfo = registry::get_model(model_id)
        .ok_or_else(|| AppError::Config(format!("Unknown model: {}", model_id)))?;

    let dir = model_dir(model_id)?;
    let total_files = info.files.len();

    for (idx, file) in info.files.iter().enumerate() {
        let dest = dir.join(file.relative_path);

        if dest.exists() && dest.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            progress_callback(DownloadProgress {
                model_id: model_id.to_string(),
                file_name: file.relative_path.to_string(),
                downloaded_bytes: file.size_bytes,
                total_bytes: file.size_bytes,
                file_index: idx,
                total_files,
            });
            continue;
        }

        let tmp_dest = dir.join(format!("{}.downloading", file.relative_path));

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .read_timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::Config(format!("Failed to create HTTP client: {}", e)))?;

        let response = client.get(file.url)
            .send()
            .await
            .map_err(|e| AppError::Config(format!("Failed to download {}: {}", file.relative_path, e)))?;

        if !response.status().is_success() {
            return Err(AppError::Config(format!(
                "Download failed for {}: HTTP {}",
                file.relative_path,
                response.status()
            )));
        }

        let total = response.content_length().unwrap_or(file.size_bytes);

        let mut downloaded: u64 = 0;
        let model_id_owned = model_id.to_string();
        let file_name = file.relative_path.to_string();

        use tokio::io::AsyncWriteExt;
        let mut out = tokio::fs::File::create(&tmp_dest)
            .await
            .map_err(|e| AppError::Config(format!("Failed to create file: {}", e)))?;

        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                AppError::Config(format!("Download stream error: {}", e))
            })?;
            out.write_all(&chunk).await.map_err(|e| {
                AppError::Config(format!("Failed to write to file: {}", e))
            })?;
            downloaded += chunk.len() as u64;

            progress_callback(DownloadProgress {
                model_id: model_id_owned.clone(),
                file_name: file_name.clone(),
                downloaded_bytes: downloaded,
                total_bytes: total,
                file_index: idx,
                total_files,
            });
        }

        out.flush().await.map_err(|e| {
            AppError::Config(format!("Failed to flush file: {}", e))
        })?;
        drop(out);

        std::fs::rename(&tmp_dest, &dest).map_err(|e| {
            AppError::Config(format!("Failed to rename downloaded file: {}", e))
        })?;
    }

    Ok(())
}

pub fn delete_model(model_id: &str) -> Result<(), AppError> {
    let dir = match models_dir() {
        Ok(d) => d.join(model_id),
        Err(_) => return Ok(()),
    };
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .map_err(|e| AppError::Config(format!("Failed to delete model: {}", e)))?;
    }
    Ok(())
}
