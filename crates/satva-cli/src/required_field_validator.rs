use satva_core::record::Record;
use satva_core::{PipelineStage, Schema, StageError, StageResult};

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
    fn validate(&self, schema: &Schema) -> Result<(), StageError> {
        if !schema.fields.contains(&self.field) {
            return Err(StageError::validation(
                self.name(),
                &self.field,
                "field does not exist",
            ));
        }

        Ok(())
    }
}
