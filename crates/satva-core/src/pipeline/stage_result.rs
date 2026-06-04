use super::stage_error::StageError;

#[derive(Debug)]
pub enum StageResult {
    Continue,
    Skip { reason: String },
    Fail { error: StageError },
}

impl StageResult {
    pub fn continue_() -> Self {
        Self::Continue
    }
    pub fn skip(reason: impl Into<String>) -> Self {
        Self::Skip {
            reason: reason.into(),
        }
    }
    pub fn fail(error: StageError) -> Self {
        Self::Fail { error }
    }
}
