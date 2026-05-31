use super::StageResult;
use crate::record::Record;

pub trait PipelineStage: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, record: Record) -> StageResult;
}
