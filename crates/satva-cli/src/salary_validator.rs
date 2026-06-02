use satva_core::record::Record;
use satva_core::{PipelineStage, StageError, StageResult};

pub struct SalaryValidator;

impl SalaryValidator {
    pub fn new() -> Self {
        Self
    }
}

impl PipelineStage for SalaryValidator {
    fn name(&self) -> &'static str {
        "SalaryValidator"
    }
    fn execute(&self, mut record: Record) -> StageResult {
        let salary_value = record.get("salary");
        let salary_str = match salary_value {
            Some(v) => v.to_string(),
            None => {
                return StageResult::Skip {
                    record,
                    reason: "missing salary".to_string(),
                };
            }
        };
        match salary_str.parse::<i64>() {
            Ok(salary) if salary > 0 => {
                record.insert("salary", salary.into());
                StageResult::Continue(record)
            }
            _ => StageResult::Fail {
                record,
                error: StageError::execution(self.name(), "invalid salary"),
            },
        }
    }
}
