use super::Schema;
use super::StageError;
use super::StageResult;
use crate::record::Record;

pub trait PipelineStage: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, record: Record) -> StageResult;
    fn validate(&self, _schema: &Schema) -> Result<(), StageError> {
        Ok(())
    }
    fn transform_schema(&self, _schema: &mut Schema) -> Result<(), StageError> {
        Ok(())
    }
}
