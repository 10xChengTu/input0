use std::path::Path;

use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineZipformerCtcModelConfig};

use crate::errors::AppError;
use crate::models::registry::BackendKind;
use crate::stt::TranscriberBackend;

pub struct ZipformerCtcBackend {
    recognizer: OfflineRecognizer,
    model_id: String,
}

unsafe impl Send for ZipformerCtcBackend {}
unsafe impl Sync for ZipformerCtcBackend {}

impl ZipformerCtcBackend {
    pub fn new(
        model_onnx_path: &Path,
        tokens_path: &Path,
        model_id: &str,
    ) -> Result<Self, AppError> {
        if !model_onnx_path.exists() {
            return Err(AppError::Whisper(format!(
                "Zipformer CTC model file not found: {}",
                model_onnx_path.display()
            )));
        }
        if !tokens_path.exists() {
            return Err(AppError::Whisper(format!(
                "Zipformer CTC tokens file not found: {}",
                tokens_path.display()
            )));
        }

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.zipformer_ctc = OfflineZipformerCtcModelConfig {
            model: Some(model_onnx_path.to_string_lossy().into_owned()),
        };
        config.model_config.tokens = Some(tokens_path.to_string_lossy().into_owned());
        config.model_config.num_threads = 4;

        let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
            AppError::Whisper("Failed to create Zipformer CTC recognizer".to_string())
        })?;

        Ok(Self {
            recognizer,
            model_id: model_id.to_string(),
        })
    }
}

impl TranscriberBackend for ZipformerCtcBackend {
    fn transcribe(&self, audio: &[f32], _language: &str) -> Result<String, AppError> {
        if audio.is_empty() {
            return Ok(String::new());
        }

        let stream = self.recognizer.create_stream();
        stream.accept_waveform(16000, audio);

        self.recognizer.decode(&stream);

        let result = stream
            .get_result()
            .ok_or_else(|| AppError::Whisper("Failed to get Zipformer CTC result".to_string()))?;

        Ok(result.text.trim().to_string())
    }

    fn backend_kind(&self) -> BackendKind {
        BackendKind::ZipformerCtc
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}
