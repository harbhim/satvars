use satva_core::pipeline_stage::PipelineStage;
use satva_core::record::Record;
use satva_core::stage_result::StageResult;

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

    fn execute(&self, record: Record) -> StageResult {
        if record.get(&self.field).is_none() {
            return StageResult::Skip {
                record,
                reason: format!("Missing required field '{}'", self.field),
            };
        }

        StageResult::Continue(record)
    }
}
