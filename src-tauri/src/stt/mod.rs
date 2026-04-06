pub mod moonshine_backend;
pub mod paraformer_backend;
pub mod sensevoice_backend;
pub mod whisper_backend;

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
