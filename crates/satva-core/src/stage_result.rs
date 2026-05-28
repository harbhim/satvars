use crate::record::Record;
use crate::stage_error::StageError;

#[derive(Debug)]
pub enum StageResult {
    Continue(Record),
    Skip { record: Record, reason: String },
    Error(StageError),
}
