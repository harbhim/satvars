use crate::record::Record;
use crate::validation_error::ValidationError;

pub trait Validator {
    fn validate(&self, record: &Record) -> Result<(), ValidationError>;
}
