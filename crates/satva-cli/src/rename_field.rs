use satva_core::record::Record;

use satva_core::{PipelineStage, StageResult};

pub struct RenameField {
    from: String,
    to: String,
}

impl RenameField {
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
        }
    }
}

impl PipelineStage for RenameField {
    fn name(&self) -> &'static str {
        "RenameField"
    }

    fn execute(&self, record: &mut Record) -> StageResult {
        if let Some(value) = record.remove(&self.from) {
            record.insert(&self.to, value);
        }

        StageResult::Continue
    }
}
