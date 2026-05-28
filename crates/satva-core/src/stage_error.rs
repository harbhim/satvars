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

    pub fn transformation(stage: &'static str, message: &str) -> Self {
        Self::Transformation {
            stage,
            message: message.to_string(),
        }
    }

    pub fn execution(stage: &'static str, message: &str) -> Self {
        Self::Execution {
            stage,
            message: message.to_string(),
        }
    }
}
