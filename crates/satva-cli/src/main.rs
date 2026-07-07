mod employee_validation;
mod required_field_validator;

use anyhow::Result;

use employee_validation::EmployeeValidationStage;
use required_field_validator::RequiredFieldValidator;

use satva_core::{PipelineOptions, RenameFieldStage, SchemaValidation, Source, pipeline::Pipeline};
use satva_types::Schema;

use satva_io::sink::JsonSink;
use satva_io::source::JsonSource;

fn main() -> Result<()> {
    let source = JsonSource::new("employees.jsonl");

    let sample = source.read_sample(1000)?;
    let schema = Schema::infer(&sample);

    println!("Detected schema:");
    println!("{schema:#?}");

    let sink = Box::new(JsonSink::new("cleaned_employees.jsonl"));

    let mut pipeline = Pipeline::new(Box::new(source));

    pipeline.add_stage(Box::new(RequiredFieldValidator::new("first_name")));
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("department")));
    pipeline.add_stage(Box::new(SchemaValidation::new(schema)));
    pipeline.add_stage(Box::new(EmployeeValidationStage));
    pipeline.add_stage(Box::new(RenameFieldStage::new("education", "edu")));

    pipeline.set_sink(sink);

    let summary = pipeline.run(PipelineOptions::new())?;

    println!("\nPipeline Summary:");
    println!("{summary:#?}");

    Ok(())
}
