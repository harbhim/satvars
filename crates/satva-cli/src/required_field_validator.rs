use satva_core::{PipelineStage, StageContext, StageResult};
use satva_types::Record;

pub struct RequiredFieldValidator {
    field: String,
}

impl RequiredFieldValidator {
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
        }
    }
}

impl PipelineStage for RequiredFieldValidator {
    fn name(&self) -> &'static str {
        "RequiredFieldValidator"
    }

    fn execute(&self, record: &mut Record, _: &StageContext) -> StageResult {
        if record.get(&self.field).is_none() {
            return StageResult::Skip {
                reason: format!("Missing required field '{}'", self.field),
            };
        }

        StageResult::Continue
    }
}
