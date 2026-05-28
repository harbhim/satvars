#[derive(Debug, Clone)]
pub struct StageError {
    pub stage: String,
    pub message: String,
}

impl StageError {
    pub fn new(stage: &str, message: &str) -> Self {
        Self {
            stage: stage.to_string(),
            message: message.to_string(),
        }
    }
}
