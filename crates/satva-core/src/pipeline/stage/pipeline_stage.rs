use super::context::StageContext;
use super::result::StageResult;
use crate::record::Record;

pub trait PipelineStage: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, record: &mut Record, ctx: &StageContext) -> StageResult;
}
