use tauri::{command, State};

use crate::errors::AppError;
use crate::stt::SharedTranscriber;

#[command]
pub async fn transcribe_audio(
    audio: Vec<f32>,
    language: String,
    transcriber: State<'_, SharedTranscriber>,
) -> Result<String, AppError> {
    let transcriber = transcriber.inner().clone();
    tokio::task::spawn_blocking(move || {
        let guard = transcriber
            .lock()
            .map_err(|e| AppError::Whisper(format!("Transcriber mutex poisoned: {}", e)))?;
        guard.transcribe(&audio, &language)
    })
    .await
    .map_err(|e| AppError::Whisper(e.to_string()))?
}

#[command]
pub fn init_whisper_model() -> Result<(), AppError> {
    Ok(())
}

#[command]
pub fn is_whisper_model_loaded(transcriber: State<'_, SharedTranscriber>) -> bool {
    transcriber
        .lock()
        .map(|t| t.is_loaded())
        .unwrap_or(false)
}
