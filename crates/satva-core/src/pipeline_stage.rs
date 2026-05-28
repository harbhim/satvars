use crate::record::Record;
use crate::stage_result::StageResult;

pub trait PipelineStage: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, record: Record) -> StageResult;
}
