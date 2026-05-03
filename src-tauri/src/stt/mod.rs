pub mod fire_red_asr_backend;
pub mod moonshine_backend;
pub mod paraformer_backend;
pub mod sensevoice_backend;
pub mod whisper_backend;
pub mod zipformer_ctc_backend;

use std::sync::{Arc, Mutex};

use crate::errors::AppError;
use crate::models::registry::BackendKind;

pub trait TranscriberBackend: Send + Sync {
    fn transcribe(&self, audio: &[f32], language: &str) -> Result<String, AppError>;
    fn backend_kind(&self) -> BackendKind;
    fn model_id(&self) -> &str;
}

pub struct ManagedTranscriber {
    inner: Option<Box<dyn TranscriberBackend>>,
}

impl ManagedTranscriber {
    pub fn new() -> Self {
        Self { inner: None }
    }

    pub fn load(&mut self, backend: Box<dyn TranscriberBackend>) {
        self.inner = Some(backend);
    }

    pub fn unload(&mut self) {
        self.inner = None;
    }

    pub fn is_loaded(&self) -> bool {
        self.inner.is_some()
    }

    pub fn transcribe(&self, audio: &[f32], language: &str) -> Result<String, AppError> {
        let backend = self
            .inner
            .as_ref()
            .ok_or_else(|| AppError::Whisper("No STT model loaded".to_string()))?;
        backend.transcribe(audio, language)
    }

    pub fn model_id(&self) -> Option<&str> {
        self.inner.as_ref().map(|b| b.model_id())
    }

    pub fn backend_kind(&self) -> Option<BackendKind> {
        self.inner.as_ref().map(|b| b.backend_kind())
    }
}

pub type SharedTranscriber = Arc<Mutex<ManagedTranscriber>>;

pub fn new_shared_transcriber() -> SharedTranscriber {
    Arc::new(Mutex::new(ManagedTranscriber::new()))
}

/// Fold UI-level language codes to the language string the underlying STT
/// engines accept. Whisper and SenseVoice both speak `"zh"` (no variant);
/// the simplified/traditional distinction is handled at the prompt layer
/// (Whisper initial_prompt) and the LLM layer.
pub(crate) fn language_to_stt_lang(code: &str) -> &str {
    match code {
        "zh-CN" | "zh-TW" => "zh",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_to_stt_lang_folds_chinese_variants() {
        assert_eq!(language_to_stt_lang("zh-CN"), "zh");
        assert_eq!(language_to_stt_lang("zh-TW"), "zh");
        assert_eq!(language_to_stt_lang("zh"), "zh");
    }

    #[test]
    fn language_to_stt_lang_passes_through_other_codes() {
        for code in ["auto", "en", "ja", "ko", "es", "fr", "de"] {
            assert_eq!(language_to_stt_lang(code), code);
        }
    }
}
