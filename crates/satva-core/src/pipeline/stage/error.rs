use std::fmt;

#[derive(Debug, Clone)]
pub enum StageError {
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

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageError::Execution { stage, message } => {
                write!(f, "[{}] Execution error: {}", stage, message)
            }
        }
    }
}

impl std::error::Error for StageError {}
