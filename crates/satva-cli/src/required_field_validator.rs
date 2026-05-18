use satva_core::record::Record;
use satva_core::validation_error::ValidationError;
use satva_core::validator::Validator;

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

impl Validator for RequiredFieldValidator {
    fn validate(&self, record: &Record) -> Result<(), ValidationError> {
        if record.get(&self.field).is_none() {
            return Err(ValidationError {
                field: self.field.clone(),
                message: "field is required".to_string(),
            });
        }

        Ok(())
    }
}
