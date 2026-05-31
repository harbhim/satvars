mod rename_field;
mod required_field_validator;

use anyhow::Result;

use rename_field::RenameField;
use required_field_validator::RequiredFieldValidator;
use satva_core::{PipelineOptions, pipeline::Pipeline};
use satva_io::CsvSource;

fn main() -> Result<()> {
    let source = Box::new(CsvSource::new("employees.csv"));
    let mut pipeline = Pipeline::new(source);
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("Education")));
    pipeline.add_stage(Box::new(RenameField::new("Education", "Edu")));
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("Edu")));
    let summary = pipeline.run(PipelineOptions {
        collect_logs: false,
    })?;
    println!("{:#?}", summary);
    Ok(())
}
