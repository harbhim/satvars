#[derive(Debug, Clone, thiserror::Error)]
pub enum StageError {
    #[error("[{stage}] Execution error: {message}")]
    Execution {
        stage: &'static str,
        message: String,
    },
}

impl StageError {
    pub fn execution(stage: &'static str, message: &str) -> Self {
        Self::Execution {
            stage,
            message: message.to_string(),
        }
    }
}
