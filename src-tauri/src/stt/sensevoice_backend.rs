use std::path::Path;

use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig};

use crate::errors::AppError;
use crate::models::registry::BackendKind;
use crate::stt::TranscriberBackend;

pub struct SenseVoiceBackend {
    recognizer: OfflineRecognizer,
    model_id: String,
}

unsafe impl Send for SenseVoiceBackend {}
unsafe impl Sync for SenseVoiceBackend {}

impl SenseVoiceBackend {
    pub fn new(
        model_onnx_path: &Path,
        tokens_path: &Path,
        model_id: &str,
    ) -> Result<Self, AppError> {
        if !model_onnx_path.exists() {
            return Err(AppError::Whisper(format!(
                "SenseVoice model file not found: {}",
                model_onnx_path.display()
            )));
        }
        if !tokens_path.exists() {
            return Err(AppError::Whisper(format!(
                "SenseVoice tokens file not found: {}",
                tokens_path.display()
            )));
        }

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.sense_voice = OfflineSenseVoiceModelConfig {
            model: Some(model_onnx_path.to_string_lossy().into_owned()),
            language: Some("auto".into()),
            use_itn: true,
        };
        config.model_config.tokens = Some(tokens_path.to_string_lossy().into_owned());
        config.model_config.num_threads = 4;

        let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
            AppError::Whisper("Failed to create SenseVoice recognizer".to_string())
        })?;

        Ok(Self {
            recognizer,
            model_id: model_id.to_string(),
        })
    }
}

fn map_language_for_sensevoice(language: &str) -> &str {
    match language {
        "zh" | "zh-CN" | "zh-TW" => "zh",
        "en" => "en",
        "ja" => "ja",
        "ko" => "ko",
        _ => "auto",
    }
}

impl TranscriberBackend for SenseVoiceBackend {
    fn transcribe(&self, audio: &[f32], language: &str) -> Result<String, AppError> {
        if audio.is_empty() {
            return Ok(String::new());
        }

        let stream = self.recognizer.create_stream();
        stream.accept_waveform(16000, audio);

        let _sv_lang = map_language_for_sensevoice(language);

        self.recognizer.decode(&stream);

        let result = stream
            .get_result()
            .ok_or_else(|| AppError::Whisper("Failed to get SenseVoice result".to_string()))?;

        Ok(result.text.trim().to_string())
    }

    fn backend_kind(&self) -> BackendKind {
        BackendKind::SenseVoice
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}

#[cfg(test)]
mod tests {
    use super::map_language_for_sensevoice;

    #[test]
    fn map_folds_chinese_variants_to_zh() {
        assert_eq!(map_language_for_sensevoice("zh-CN"), "zh");
        assert_eq!(map_language_for_sensevoice("zh-TW"), "zh");
        assert_eq!(map_language_for_sensevoice("zh"), "zh");
    }

    #[test]
    fn map_passes_through_supported_codes() {
        assert_eq!(map_language_for_sensevoice("en"), "en");
        assert_eq!(map_language_for_sensevoice("ja"), "ja");
        assert_eq!(map_language_for_sensevoice("ko"), "ko");
    }

    #[test]
    fn map_unknown_codes_to_auto() {
        assert_eq!(map_language_for_sensevoice("auto"), "auto");
        assert_eq!(map_language_for_sensevoice("es"), "auto");
    }
}
