use satva_core::record::Record;
use satva_core::{PipelineStage, StageContext, StageError, StageResult};

pub struct AgeValidator;

impl AgeValidator {
    pub fn new() -> Self {
        Self
    }
}

impl PipelineStage for AgeValidator {
    fn name(&self) -> &'static str {
        "AgeValidator"
    }

    fn execute(&self, record: &mut Record, _: &StageContext) -> StageResult {
        let age_value = match record.get("age") {
            Some(value) => value,
            None => {
                return StageResult::Skip {
                    reason: "missing age".to_string(),
                };
            }
        };

        let age = match age_value.as_string() {
            Some(value) => match value.parse::<i64>() {
                Ok(age) => age,
                Err(_) => {
                    return StageResult::Fail {
                        error: StageError::execution(self.name(), "age is not a valid integer"),
                    };
                }
            },
            None => {
                return StageResult::Fail {
                    error: StageError::execution(self.name(), "age must be a string"),
                };
            }
        };

        if age <= 0 {
            return StageResult::Fail {
                error: StageError::execution(self.name(), "age must be greater than zero"),
            };
        }

        record.insert("age", age.into());

        StageResult::Continue
    }
}
