use serde::Serialize;
use tauri::{command, AppHandle, Emitter, State};

use crate::config;
use crate::errors::AppError;
use crate::models::{manager, registry};
use crate::stt::SharedTranscriber;

#[derive(Debug, Clone, Serialize)]
pub struct RecommendedModel {
    pub id: String,
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelRecommendation {
    pub should_switch: bool,
    pub recommended_models: Vec<RecommendedModel>,
    pub current_model_id: String,
}

#[command]
pub fn list_models(transcriber: State<'_, SharedTranscriber>) -> Result<Vec<registry::ModelInfoDto>, AppError> {
    let active_model_id = transcriber
        .lock()
        .map_err(|e| AppError::Whisper(format!("Transcriber mutex poisoned: {}", e)))?
        .model_id()
        .unwrap_or("whisper-base")
        .to_string();

    Ok(manager::list_models_with_status(&active_model_id))
}

#[command]
pub async fn download_model(model_id: String, app: AppHandle) -> Result<(), AppError> {
    registry::get_model(&model_id)
        .ok_or_else(|| AppError::Config(format!("Unknown model: {}", model_id)))?;

    let app_clone = app.clone();
    let model_id_clone = model_id.clone();

    manager::download_model(&model_id_clone, move |progress| {
        let _ = app_clone.emit("model-download-progress", &progress);
    })
    .await?;

    Ok(())
}

#[command]
pub async fn switch_model(
    model_id: String,
    transcriber: State<'_, SharedTranscriber>,
) -> Result<(), AppError> {
    let info = registry::get_model(&model_id)
        .ok_or_else(|| AppError::Config(format!("Unknown model: {}", model_id)))?;

    if !manager::is_model_downloaded(&model_id) {
        return Err(AppError::Config(format!(
            "Model '{}' is not downloaded yet. Please download it first.",
            info.display_name
        )));
    }

    let backend: Box<dyn crate::stt::TranscriberBackend> = match info.backend {
        registry::BackendKind::Whisper => {
            let path = manager::whisper_model_path(&model_id)?;
            Box::new(crate::stt::whisper_backend::WhisperBackend::new(&path, &model_id)?)
        }
        registry::BackendKind::SenseVoice => {
            let (onnx, tokens) = manager::sensevoice_model_paths(&model_id)?;
            Box::new(crate::stt::sensevoice_backend::SenseVoiceBackend::new(&onnx, &tokens, &model_id)?)
        }
        registry::BackendKind::Paraformer => {
            let (onnx, tokens) = manager::paraformer_model_paths(&model_id)?;
            Box::new(crate::stt::paraformer_backend::ParaformerBackend::new(&onnx, &tokens, &model_id)?)
        }
        registry::BackendKind::Moonshine => {
            let (preprocessor, encoder, uncached_decoder, cached_decoder, tokens) =
                manager::moonshine_model_paths(&model_id)?;
            Box::new(crate::stt::moonshine_backend::MoonshineBackend::new(
                &preprocessor, &encoder, &uncached_decoder, &cached_decoder, &tokens, &model_id,
            )?)
        }
    };

    let mut guard = transcriber
        .lock()
        .map_err(|e| AppError::Whisper(format!("Transcriber mutex poisoned: {}", e)))?;
    guard.load(backend);

    config::update_field("stt_model", &model_id)?;

    Ok(())
}

#[command]
pub fn delete_model(
    model_id: String,
    transcriber: State<'_, SharedTranscriber>,
) -> Result<(), AppError> {
    let active_model_id = transcriber
        .lock()
        .map_err(|e| AppError::Whisper(format!("Transcriber mutex poisoned: {}", e)))?
        .model_id()
        .map(|s| s.to_string());

    if active_model_id.as_deref() == Some(model_id.as_str()) {
        return Err(AppError::Config(
            "Cannot delete the currently active model. Please switch to another model first.".to_string(),
        ));
    }

    manager::delete_model(&model_id)
}

#[command]
pub fn get_model_recommendation(
    language: String,
    transcriber: State<'_, SharedTranscriber>,
) -> Result<ModelRecommendation, AppError> {
    let current_model_id = transcriber
        .lock()
        .map_err(|e| AppError::Whisper(format!("Transcriber mutex poisoned: {}", e)))?
        .model_id()
        .unwrap_or("whisper-base")
        .to_string();

    match registry::suggest_model_switch(&current_model_id, &language) {
        Some(recommendations) => {
            let recommended_models = recommendations
                .into_iter()
                .map(|(id, name, reason)| RecommendedModel {
                    id: id.to_string(),
                    name: name.to_string(),
                    reason: reason.to_string(),
                })
                .collect();
            Ok(ModelRecommendation {
                should_switch: true,
                recommended_models,
                current_model_id,
            })
        }
        None => Ok(ModelRecommendation {
            should_switch: false,
            recommended_models: vec![],
            current_model_id,
        }),
    }
}
