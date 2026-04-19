use std::path::PathBuf;

use serde::Serialize;

use crate::config;
use crate::errors::AppError;
use crate::models::registry::{self, ModelInfo, ModelInfoDto};

const MAX_RETRIES: u32 = 3;
const CONNECT_TIMEOUT_SECS: u64 = 30;
const READ_TIMEOUT_SECS: u64 = 300;

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

pub fn fire_red_asr_model_paths(model_id: &str) -> Result<(PathBuf, PathBuf, PathBuf), AppError> {
    let dir = model_dir(model_id)?;
    let encoder = dir.join("encoder.int8.onnx");
    let decoder = dir.join("decoder.int8.onnx");
    let tokens = dir.join("tokens.txt");
    Ok((encoder, decoder, tokens))
}

pub fn zipformer_ctc_model_paths(model_id: &str) -> Result<(PathBuf, PathBuf), AppError> {
    let dir = model_dir(model_id)?;
    let model = dir.join("model.int8.onnx");
    let tokens = dir.join("tokens.txt");
    Ok((model, tokens))
}

fn build_http_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .read_timeout(std::time::Duration::from_secs(READ_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Config(format!("Failed to create HTTP client: {}", e)))
}

async fn download_file_with_resume(
    client: &reqwest::Client,
    url: &str,
    tmp_dest: &std::path::Path,
    expected_size: u64,
    model_id: &str,
    file_name: &str,
    file_index: usize,
    total_files: usize,
    progress_callback: &(dyn Fn(DownloadProgress) + Send + Sync),
) -> Result<(), AppError> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let mut downloaded: u64 = if tmp_dest.exists() {
        tmp_dest.metadata().map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    let mut request = client.get(url);
    if downloaded > 0 {
        request = request.header("Range", format!("bytes={}-", downloaded));
    }

    let response = request
        .send()
        .await
        .map_err(|e| AppError::Config(format!("Failed to download {}: {}", file_name, e)))?;

    let status = response.status();

    if downloaded > 0 && status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        return Ok(());
    }

    if downloaded > 0 && status == reqwest::StatusCode::OK {
        downloaded = 0;
    }

    if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(AppError::Config(format!(
            "Download failed for {}: HTTP {}",
            file_name, status
        )));
    }

    let total = if status == reqwest::StatusCode::PARTIAL_CONTENT {
        downloaded + response.content_length().unwrap_or(expected_size.saturating_sub(downloaded))
    } else {
        response.content_length().unwrap_or(expected_size)
    };

    let append = downloaded > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT;
    let mut out = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(tmp_dest)
        .await
        .map_err(|e| AppError::Config(format!("Failed to open file: {}", e)))?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            AppError::Config(format!("Download stream error: {}", e))
        })?;
        out.write_all(&chunk).await.map_err(|e| {
            AppError::Config(format!("Failed to write to file: {}", e))
        })?;
        downloaded += chunk.len() as u64;

        progress_callback(DownloadProgress {
            model_id: model_id.to_string(),
            file_name: file_name.to_string(),
            downloaded_bytes: downloaded,
            total_bytes: total,
            file_index,
            total_files,
        });
    }

    out.flush().await.map_err(|e| {
        AppError::Config(format!("Failed to flush file: {}", e))
    })?;

    Ok(())
}

pub async fn download_model(
    model_id: &str,
    progress_callback: impl Fn(DownloadProgress) + Send + Sync + 'static,
) -> Result<(), AppError> {
    let info: &ModelInfo = registry::get_model(model_id)
        .ok_or_else(|| AppError::Config(format!("Unknown model: {}", model_id)))?;

    let hf_endpoint = config::load()
        .map(|c| c.hf_endpoint)
        .unwrap_or_default();

    let dir = model_dir(model_id)?;
    let total_files = info.files.len();
    let client = build_http_client()?;

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

        let url = registry::resolve_url(file.url, &hf_endpoint);
        let tmp_dest = dir.join(format!("{}.downloading", file.relative_path));

        let mut last_error = None;
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt));
                tokio::time::sleep(delay).await;
            }

            match download_file_with_resume(
                &client,
                &url,
                &tmp_dest,
                file.size_bytes,
                model_id,
                file.relative_path,
                idx,
                total_files,
                &progress_callback,
            )
            .await
            {
                Ok(()) => {
                    last_error = None;
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        if let Some(e) = last_error {
            return Err(e);
        }

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
