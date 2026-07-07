use satva_core::PipelineStage;
use satva_core::{StageContext, StageError, StageResult};
use satva_types::{Record, Value};

pub struct EmployeeValidationStage;

impl PipelineStage for EmployeeValidationStage {
    fn name(&self) -> &'static str {
        "EmployeeValidationStage"
    }
    fn execute(&self, record: &mut Record, _ctx: &StageContext) -> StageResult {
        match self.validate_age(record) {
            StageResult::Continue => {}
            result => return result,
        }

        match self.validate_salary(record) {
            StageResult::Continue => {}
            result => return result,
        }

        self.enrich_experience(record)
    }
}

impl EmployeeValidationStage {
    fn enrich_experience(&self, record: &mut Record) -> StageResult {
        let exp = record.get("experience_years");
        let years = match exp {
            Some(Value::Int64(v)) => *v,
            _ => 0,
        };

        let level = if years >= 8 {
            "Senior"
        } else if years >= 3 {
            "Mid"
        } else {
            "Junior"
        };

        record.insert("seniority_level", level.into());

        StageResult::Continue
    }

    fn validate_age(&self, record: &mut Record) -> StageResult {
        let Some(age_value) = record.get("age") else {
            return StageResult::Skip {
                reason: "missing age".to_string(),
            };
        };
        let Some(age) = age_value.as_i64() else {
            return StageResult::Fail {
                error: StageError::execution(self.name(), "age must be an integer"),
            };
        };
        if age <= 0 {
            return StageResult::Fail {
                error: StageError::execution(self.name(), "age must be greater than zero"),
            };
        }
        StageResult::Continue
    }

    fn validate_salary(&self, record: &mut Record) -> StageResult {
        let salary_value = record.get("salary");
        let salary = match salary_value {
            Some(Value::Int64(s)) => *s,
            _ => {
                return StageResult::Fail {
                    error: StageError::execution(self.name(), "invalid salary"),
                };
            }
        };
        if salary <= 0 {
            return StageResult::Fail {
                error: StageError::execution(self.name(), "invalid salary"),
            };
        }
        StageResult::Continue
    }
}
