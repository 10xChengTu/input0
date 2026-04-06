use std::path::Path;
use std::sync::OnceLock;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::errors::AppError;

// Global singleton for the WhisperContext (expensive to create, thread-safe)
static WHISPER_CONTEXT: OnceLock<WhisperContext> = OnceLock::new();

/// Initialize the whisper model. Call this once at app startup.
/// model_path: path to the .bin model file (e.g., ggml-base.bin)
pub fn init_model(model_path: &Path) -> Result<(), AppError> {
    if WHISPER_CONTEXT.get().is_some() {
        return Err(AppError::Whisper("Model already initialized".to_string()));
    }

    if !model_path.exists() {
        return Err(AppError::Whisper(format!(
            "Model file not found: {}",
            model_path.display()
        )));
    }

    let path_str = model_path
        .to_str()
        .ok_or_else(|| AppError::Whisper("Invalid model path encoding".to_string()))?;

    let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
        .map_err(|e| AppError::Whisper(format!("Failed to load whisper model: {}", e)))?;

    WHISPER_CONTEXT
        .set(ctx)
        .map_err(|_| AppError::Whisper("Failed to store whisper context".to_string()))?;

    Ok(())
}

/// Check if the model is loaded
pub fn is_model_loaded() -> bool {
    WHISPER_CONTEXT.get().is_some()
}

pub(crate) fn initial_prompt_for_language(language: &str) -> Option<&'static str> {
    match language {
        "zh" => Some("以下是普通话的句子。"),
        _ => None,
    }
}

/// Transcribe audio samples (mono, f32, 16kHz) to text
/// language: ISO code like "en", "zh", "auto"
pub fn transcribe(audio: &[f32], language: &str) -> Result<String, AppError> {
    let ctx = WHISPER_CONTEXT.get().ok_or_else(|| {
        AppError::Whisper("Whisper model not initialized. Call init_model first.".to_string())
    })?;

    if audio.is_empty() {
        return Ok(String::new());
    }

    let mut state = ctx
        .create_state()
        .map_err(|e| AppError::Whisper(format!("Failed to create whisper state: {}", e)))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    if language != "auto" {
        params.set_language(Some(language));
    } else {
        params.set_language(None);
    }

    // Use initial_prompt to guide Whisper output script (simplified vs traditional).
    // Whisper's "zh" language code doesn't distinguish simplified/traditional Chinese;
    // setting an initial_prompt with simplified characters biases the model toward
    // simplified output. See: https://github.com/openai/whisper/discussions/277
    let initial_prompt = initial_prompt_for_language(language);
    if let Some(prompt) = initial_prompt {
        params.set_initial_prompt(prompt);
    }

    params.set_translate(false);
    params.set_no_timestamps(true);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);

    state
        .full(params, audio)
        .map_err(|e| AppError::Whisper(format!("Transcription failed: {}", e)))?;

    let num_segments = state
        .full_n_segments()
        .map_err(|e| AppError::Whisper(format!("Failed to get segment count: {}", e)))?;

    let mut result = String::new();
    for i in 0..num_segments {
        let text = state
            .full_get_segment_text(i)
            .map_err(|e| AppError::Whisper(format!("Failed to get segment {}: {}", i, e)))?;
        result.push_str(&text);
    }

    Ok(result.trim().to_string())
}
