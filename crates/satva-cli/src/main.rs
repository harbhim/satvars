mod mock_source;
mod rename_field;
mod required_field_validator;

use anyhow::Result;

use mock_source::MockSource;
use rename_field::RenameField;
use required_field_validator::RequiredFieldValidator;
use satva_core::pipeline::Pipeline;
use satva_io::CsvSource;

fn main() -> Result<()> {
    // let source = Box::new(MockSource);
    let source = Box::new(CsvSource::new("employees.csv"));
    let mut pipeline = Pipeline::new(source);
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("fname")));
    pipeline.add_stage(Box::new(RenameField::new("fname", "first_name")));
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("first_name")));
    let summary = pipeline.run()?;
    println!("{:#?}", summary);
    Ok(())
}
