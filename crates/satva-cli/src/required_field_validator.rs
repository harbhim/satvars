use satva_core::pipeline_stage::PipelineStage;
use satva_core::record::Record;
use satva_core::stage_error::StageError;
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
    fn name(&self) -> &str {
        "RequiredFieldValidator"
    }

    fn execute(&self, record: Record) -> StageResult {
        if record.get(&self.field).is_none() {
            return StageResult::Skip {
                record,

                reason: StageError::new(
                    self.name(),
                    &format!("Missing required field '{}'", self.field),
                ),
            };
        }

        StageResult::Continue(record)
    }
}
