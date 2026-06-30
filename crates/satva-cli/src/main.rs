mod employee_validation;
mod rename_field;
mod required_field_validator;

use anyhow::Result;

use employee_validation::EmployeeValidationStage;
use rename_field::RenameField;
use required_field_validator::RequiredFieldValidator;
use satva_core::{PipelineOptions, pipeline::Pipeline};
use satva_io::{CsvSink, CsvSource};

fn main() -> Result<()> {
    let source = Box::new(CsvSource::new("employees.csv"));
    let sink = Box::new(CsvSink::new("cleaned_employees.csv"));

    let mut pipeline = Pipeline::new(source);
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("first_name")));
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("department")));
    pipeline.add_stage(Box::new(EmployeeValidationStage));
    pipeline.add_stage(Box::new(RenameField::new("education", "edu")));
    pipeline.set_sink(sink);

    let summary = pipeline.run(PipelineOptions { collect_logs: true })?;
    println!("{:#?}", summary);
    Ok(())
}
