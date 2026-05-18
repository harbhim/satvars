use anyhow::Result;

use crate::source::Source;
use crate::transformer::Transformer;
use crate::validator::Validator;

pub struct Pipeline {
    source: Box<dyn Source>,

    transformers: Vec<Box<dyn Transformer>>,

    validators: Vec<Box<dyn Validator>>,
}

impl Pipeline {
    pub fn new(source: Box<dyn Source>) -> Self {
        Self {
            source,
            transformers: Vec::new(),
            validators: Vec::new(),
        }
    }

    pub fn add_transformer(&mut self, transformer: Box<dyn Transformer>) {
        self.transformers.push(transformer);
    }

    pub fn add_validator(&mut self, validator: Box<dyn Validator>) {
        self.validators.push(validator);
    }

    pub fn run(&self) -> Result<()> {
        let records = self.source.read()?;

        for mut record in records {
            for transformer in &self.transformers {
                record = transformer.transform(record)?;
            }

            let mut valid = true;

            for validator in &self.validators {
                if let Err(error) = validator.validate(&record) {
                    println!("VALIDATION ERROR:\n{:#?}", error);

                    valid = false;

                    break;
                }
            }

            if valid {
                println!("VALID RECORD:\n{:#?}", record);
            }
        }

        Ok(())
    }
}
