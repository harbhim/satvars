use std::fmt;

#[derive(Debug, Clone)]
pub enum StageError {
    Validation {
        stage: &'static str,
        field: String,
        message: String,
    },

    Transformation {
        stage: &'static str,
        message: String,
    },

    Execution {
        stage: &'static str,
        message: String,
    },
}

impl StageError {
    pub fn validation(stage: &'static str, field: &str, message: &str) -> Self {
        Self::Validation {
            stage,
            field: field.to_string(),
            message: message.to_string(),
        }
    }
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StageError::Validation {
                stage,
                field,
                message,
            } => write!(
                f,
                "[{}] Validation error on field '{}': {}",
                stage, field, message
            ),

            StageError::Transformation { stage, message } => {
                write!(f, "[{}] Transformation error: {}", stage, message)
            }

            StageError::Execution { stage, message } => {
                write!(f, "[{}] Execution error: {}", stage, message)
            }
        }
    }
}

impl std::error::Error for StageError {}
