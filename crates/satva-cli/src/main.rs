mod employee_validation;
mod rename_field;
mod required_field_validator;

use anyhow::Result;

use employee_validation::EmployeeValidationStage;
use rename_field::RenameField;
use required_field_validator::RequiredFieldValidator;
use satva_core::{PipelineOptions, Schema, SchemaValidation, Source, pipeline::Pipeline};
use satva_io::{CsvSink, CsvSource};

fn main() -> Result<()> {
    let source = CsvSource::new("employees.csv");
    let sample = source.read_sample(1000)?;
    let schema = Schema::infer(&sample);

    let sink = Box::new(CsvSink::new("cleaned_employees.csv"));

    let mut pipeline = Pipeline::new(Box::new(source));
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("first_name")));
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("department")));
    pipeline.add_stage(Box::new(SchemaValidation::new(schema)));
    pipeline.add_stage(Box::new(EmployeeValidationStage));
    pipeline.add_stage(Box::new(RenameField::new("education", "edu")));
    pipeline.set_sink(sink);

    let summary = pipeline.run(PipelineOptions::new())?;
    println!("{summary:#?}");
    Ok(())
}
