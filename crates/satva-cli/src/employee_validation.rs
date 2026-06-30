use satva_core::PipelineStage;
use satva_core::{StageContext, StageError, StageResult, record::Record};

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
            Some(v) => v.to_string().parse::<i32>().unwrap_or(0),
            None => 0,
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

    fn validate_salary(&self, record: &mut Record) -> StageResult {
        let salary_value = record.get("salary");
        let salary_str = match salary_value {
            Some(v) => v.to_string(),
            None => {
                return StageResult::Skip {
                    reason: "missing salary".to_string(),
                };
            }
        };
        match salary_str.parse::<i64>() {
            Ok(salary) if salary > 0 => {
                record.insert("salary", salary.into());
                StageResult::Continue
            }
            _ => StageResult::Fail {
                error: StageError::execution(self.name(), "invalid salary"),
            },
        }
    }
}
