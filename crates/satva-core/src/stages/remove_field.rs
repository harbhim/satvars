use satva_types::Record;

use crate::{PipelineStage, StageContext, StageResult};

pub struct RemoveFieldStage {
    fields: Box<[String]>,
}

impl RemoveFieldStage {
    pub fn new<I, S>(fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            fields: fields.into_iter().map(Into::into).collect(),
        }
    }
}

impl PipelineStage for RemoveFieldStage {
    fn name(&self) -> &'static str {
        "RemoveField"
    }

    fn execute(&self, record: &mut Record, _ctx: &StageContext) -> StageResult {
        for field in &self.fields {
            record.remove(field);
        }

        StageResult::continue_()
    }
}
