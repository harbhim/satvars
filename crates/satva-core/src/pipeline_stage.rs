use crate::record::Record;
use crate::stage_result::StageResult;

pub trait PipelineStage {
    fn name(&self) -> &str;
    fn execute(&self, record: Record) -> StageResult;
}
