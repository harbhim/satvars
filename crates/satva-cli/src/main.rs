mod mock_source;
mod rename_field;
mod required_field_validator;

use anyhow::Result;

use satva_core::pipeline::Pipeline;

use mock_source::MockSource;
use rename_field::RenameField;
use required_field_validator::RequiredFieldValidator;

fn main() -> Result<()> {
    let source = Box::new(MockSource);

    let mut pipeline = Pipeline::new(source);

    pipeline.add_transformer(Box::new(RenameField::new("fname", "first_name")));

    pipeline.add_validator(Box::new(RequiredFieldValidator::new("first_name")));

    pipeline.run()?;

    Ok(())
}
