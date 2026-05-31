use anyhow::Result;
use anyhow::anyhow;

use crate::pipeline_stage::PipelineStage;
use crate::source::Source;
use crate::stage_result::StageResult;

#[derive(Debug)]
pub struct PipelineSummary {
    pub processed: usize,
    pub succeeded: usize,
    pub skipped: usize,
    pub errors: usize,
}

pub struct Pipeline {
    source: Box<dyn Source>,
    stages: Vec<Box<dyn PipelineStage>>,
}

impl Pipeline {
    pub fn new(source: Box<dyn Source>) -> Self {
        Self {
            source,
            stages: Vec::new(),
        }
    }

    pub fn add_stage(&mut self, stage: Box<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    pub fn run(&self) -> Result<PipelineSummary> {
        let records = self.source.read()?;

        let mut summary = PipelineSummary {
            processed: 0,
            succeeded: 0,
            skipped: 0,
            errors: 0,
        };

        for record in records {
            summary.processed += 1;
            let mut current_record = Some(record);
            for stage in &self.stages {
                let record = current_record.take().expect("record should exist");
                match stage.execute(record) {
                    StageResult::Continue(record) => {
                        current_record = Some(record);
                    }
                    StageResult::Skip { .. } => {
                        summary.skipped += 1;
                        current_record = None;
                        break;
                    }
                    StageResult::Error(error) => return Err(anyhow!(error)),
                }
            }
            if let Some(_) = current_record {
                summary.succeeded += 1;
            }
        }
        Ok(summary)
    }
}
