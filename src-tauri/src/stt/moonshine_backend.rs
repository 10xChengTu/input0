use std::path::Path;

use sherpa_onnx::{OfflineMoonshineModelConfig, OfflineRecognizer, OfflineRecognizerConfig};

use crate::errors::AppError;
use crate::models::registry::BackendKind;
use crate::stt::TranscriberBackend;

pub struct MoonshineBackend {
    recognizer: OfflineRecognizer,
    model_id: String,
}

unsafe impl Send for MoonshineBackend {}
unsafe impl Sync for MoonshineBackend {}

impl MoonshineBackend {
    pub fn new(
        preprocessor_path: &Path,
        encoder_path: &Path,
        uncached_decoder_path: &Path,
        cached_decoder_path: &Path,
        tokens_path: &Path,
        model_id: &str,
    ) -> Result<Self, AppError> {
        for (label, path) in [
            ("preprocessor", preprocessor_path),
            ("encoder", encoder_path),
            ("uncached_decoder", uncached_decoder_path),
            ("cached_decoder", cached_decoder_path),
            ("tokens", tokens_path),
        ] {
            if !path.exists() {
                return Err(AppError::Whisper(format!(
                    "Moonshine {} file not found: {}",
                    label,
                    path.display()
                )));
            }
        }

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.moonshine = OfflineMoonshineModelConfig {
            preprocessor: Some(preprocessor_path.to_string_lossy().into_owned()),
            encoder: Some(encoder_path.to_string_lossy().into_owned()),
            uncached_decoder: Some(uncached_decoder_path.to_string_lossy().into_owned()),
            cached_decoder: Some(cached_decoder_path.to_string_lossy().into_owned()),
            merged_decoder: None,
        };
        config.model_config.tokens = Some(tokens_path.to_string_lossy().into_owned());
        config.model_config.num_threads = 4;

        let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
            AppError::Whisper("Failed to create Moonshine recognizer".to_string())
        })?;

        Ok(Self {
            recognizer,
            model_id: model_id.to_string(),
        })
    }
}

impl TranscriberBackend for MoonshineBackend {
    fn transcribe(&self, audio: &[f32], _language: &str) -> Result<String, AppError> {
        if audio.is_empty() {
            return Ok(String::new());
        }

        let stream = self.recognizer.create_stream();
        stream.accept_waveform(16000, audio);

        self.recognizer.decode(&stream);

        let result = stream
            .get_result()
            .ok_or_else(|| AppError::Whisper("Failed to get Moonshine result".to_string()))?;

        Ok(result.text.trim().to_string())
    }

    fn backend_kind(&self) -> BackendKind {
        BackendKind::Moonshine
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}
