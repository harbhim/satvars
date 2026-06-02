use super::stage_error::StageError;

#[derive(Debug)]
pub enum StageResult {
    Continue,
    Skip { reason: String },
    Fail { error: StageError },
}
