use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Audio error: {0}")]
    Audio(String),
    #[error("Whisper error: {0}")]
    Whisper(String),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Input error: {0}")]
    Input(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Vocabulary error: {0}")]
    Vocabulary(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}
