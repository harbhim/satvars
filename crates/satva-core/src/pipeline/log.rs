use super::StageError;

#[derive(Debug, Clone)]
pub enum PipelineLog {
    Skipped {
        record_index: usize,
        stage: &'static str,
        reason: String,
    },
    StageFailed {
        record_index: usize,
        error: StageError,
    },
    SinkFailed {
        record_index: usize,
        message: String,
    },
}
