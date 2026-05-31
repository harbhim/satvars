use crate::record::Record;

use super::stage_error::StageError;

#[derive(Debug)]
pub enum StageResult {
    Continue(Record),
    Skip { record: Record, reason: String },
    Fail { record: Record, error: StageError },
}
