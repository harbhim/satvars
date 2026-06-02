use satva_core::record::Record;
use satva_core::{PipelineStage, StageResult};

pub struct ExperienceEnrichment;

impl ExperienceEnrichment {
    pub fn new() -> Self {
        Self
    }
}

impl PipelineStage for ExperienceEnrichment {
    fn name(&self) -> &'static str {
        "ExperienceEnrichment"
    }
    fn execute(&self, mut record: Record) -> StageResult {
        let exp = record.get("experience_level");

        let years = match exp {
            Some(v) => v.to_string().parse::<i32>().unwrap_or(0),
            None => 0,
        };

        let level = if years >= 8 {
            "Senior"
        } else if years >= 3 {
            "Mid"
        } else {
            "Junior"
        };

        record.insert("seniority_level", level.into());

        StageResult::Continue(record)
    }
}
