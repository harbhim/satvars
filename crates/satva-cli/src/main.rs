mod employee_validation;
mod rename_field;
mod required_field_validator;

use anyhow::Result;

use employee_validation::EmployeeValidationStage;
use rename_field::RenameField;
use required_field_validator::RequiredFieldValidator;
use satva_core::{
    DataType, Field, PipelineOptions, Schema, SchemaValidation, pipeline::Pipeline,
};
use satva_io::{CsvSink, CsvSource};

fn main() -> Result<()> {
    let source = Box::new(CsvSource::new("employees.csv"));
    let sink = Box::new(CsvSink::new("cleaned_employees.csv"));

    let schema = Schema::new(vec![
        Field::new("employee_id", DataType::Int64, false),
        Field::new("first_name", DataType::String, false),
        Field::new("last_name", DataType::String, false),
        Field::new("age", DataType::Int64, false),
        Field::new("department", DataType::String, false),
        Field::new("salary", DataType::Int64, false),
        Field::new("education", DataType::String, true),
        Field::new("city", DataType::String, false),
        Field::new("experience_years", DataType::Int64, false),
    ]);

    let mut pipeline = Pipeline::new(source);
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("first_name")));
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("department")));
    pipeline.add_stage(Box::new(SchemaValidation::new(schema)));
    pipeline.add_stage(Box::new(EmployeeValidationStage));
    pipeline.add_stage(Box::new(RenameField::new("education", "edu")));
    pipeline.set_sink(sink);

    let summary = pipeline.run(PipelineOptions { collect_logs: true })?;
    println!("{summary:#?}");
    Ok(())
}
