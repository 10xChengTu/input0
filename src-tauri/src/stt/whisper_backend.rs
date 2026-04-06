use std::path::Path;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::errors::AppError;
use crate::models::registry::BackendKind;
use crate::stt::TranscriberBackend;

pub struct WhisperBackend {
    ctx: WhisperContext,
    model_id: String,
}

impl WhisperBackend {
    pub fn new(model_path: &Path, model_id: &str) -> Result<Self, AppError> {
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

        Ok(Self {
            ctx,
            model_id: model_id.to_string(),
        })
    }
}

fn initial_prompt_for_language(language: &str) -> Option<&'static str> {
    match language {
        "zh" => Some("以下是普通话的句子。"),
        _ => None,
    }
}

impl TranscriberBackend for WhisperBackend {
    fn transcribe(&self, audio: &[f32], language: &str) -> Result<String, AppError> {
        if audio.is_empty() {
            return Ok(String::new());
        }

        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| AppError::Whisper(format!("Failed to create whisper state: {}", e)))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        if language != "auto" {
            params.set_language(Some(language));
        } else {
            params.set_language(None);
        }

        if let Some(prompt) = initial_prompt_for_language(language) {
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

    fn backend_kind(&self) -> BackendKind {
        BackendKind::Whisper
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}
