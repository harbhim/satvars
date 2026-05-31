use super::stage_error::StageError;

#[derive(Debug, Clone)]
pub enum PipelineLog {
    Skipped { stage: &'static str, reason: String },
    Failed { error: StageError },
}
