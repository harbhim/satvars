use super::StageError;

#[derive(Debug, Clone)]
pub enum PipelineLog {
    Skipped {
        record_index: usize,
        stage: &'static str,
        reason: String,
    },
    Failed {
        record_index: usize,
        error: StageError,
    },
}
