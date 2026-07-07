use satva_types::Record;

use crate::{PipelineStage, StageContext, StageResult};

pub struct RenameFieldStage {
    from: String,
    to: String,
}

impl RenameFieldStage {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

impl PipelineStage for RenameFieldStage {
    fn name(&self) -> &'static str {
        "RenameField"
    }

    fn execute(&self, record: &mut Record, _ctx: &StageContext) -> StageResult {
        if let Some(value) = record.remove(&self.from) {
            record.insert(&self.to, value);
        }

        StageResult::continue_()
    }
}
