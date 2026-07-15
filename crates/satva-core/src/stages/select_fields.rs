use std::collections::HashSet;

use satva_types::Record;

use crate::{PipelineStage, StageContext, StageResult};

pub struct SelectFieldsStage {
    fields: HashSet<String>,
}

impl SelectFieldsStage {
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

impl PipelineStage for SelectFieldsStage {
    fn name(&self) -> &'static str {
        "SelectFields"
    }

    fn execute(&self, record: &mut Record, _ctx: &StageContext) -> StageResult {
        record.fields.retain(|field, _| self.fields.contains(field));

        StageResult::continue_()
    }
}
