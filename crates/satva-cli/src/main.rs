mod age_validator;
mod experience_enrichment;
mod rename_field;
mod required_field_validator;
mod salary_validator;

use anyhow::Result;

use age_validator::AgeValidator;
use experience_enrichment::ExperienceEnrichment;
use rename_field::RenameField;
use required_field_validator::RequiredFieldValidator;
use satva_core::{PipelineOptions, pipeline::Pipeline};
use satva_io::CsvSource;

use salary_validator::SalaryValidator;

fn main() -> Result<()> {
    let source = Box::new(CsvSource::new("employees.csv"));
    let mut pipeline = Pipeline::new(source);
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("first_name")));
    pipeline.add_stage(Box::new(RequiredFieldValidator::new("department")));
    pipeline.add_stage(Box::new(AgeValidator::new()));
    pipeline.add_stage(Box::new(SalaryValidator::new()));
    pipeline.add_stage(Box::new(RenameField::new("education", "edu")));
    pipeline.add_stage(Box::new(ExperienceEnrichment::new()));
    let summary = pipeline.run(PipelineOptions { collect_logs: true })?;
    println!("{:#?}", summary);
    Ok(())
}
