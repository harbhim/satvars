#[derive(Debug, Clone, thiserror::Error)]
pub enum StageError {
    #[error("[{stage}] Execution error: {message}")]
    Execution { stage: String, message: String },
}

impl StageError {
    pub fn execution(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Execution {
            stage: stage.into(),
            message: message.into(),
        }
    }
}
