use crate::source::Source;
use anyhow::Result;

use super::{
    PipelineLog, PipelineOptions, PipelineRunResult, PipelineStage, PipelineSummary, StageResult,
};

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

    pub fn run(&self, options: PipelineOptions) -> Result<PipelineRunResult> {
        let mut schema = self.source.schema()?;

        for stage in &self.stages {
            stage.validate(&schema)?;
            stage.transform_schema(&mut schema)?;
        }

        let records = self.source.read()?;

        let mut summary = PipelineSummary::default();
        let mut logs = Vec::new();

        for record in records {
            summary.processed += 1;

            let mut current_record = Some(record);

            for stage in &self.stages {
                let record = current_record.take().expect("record should exist");

                match stage.execute(record) {
                    StageResult::Continue(record) => {
                        current_record = Some(record);
                    }

                    StageResult::Skip { reason, .. } => {
                        summary.skipped += 1;

                        logs.push(PipelineLog::Skipped {
                            stage: stage.name(),
                            reason,
                        });

                        current_record = None;
                        break;
                    }

                    StageResult::Fail { error, .. } => {
                        summary.failed += 1;

                        if options.collect_logs {
                            logs.push(PipelineLog::Failed { error });
                        }

                        current_record = None;
                        break;
                    }
                }
            }

            if current_record.is_some() {
                summary.succeeded += 1;
            }
        }

        Ok(PipelineRunResult { summary, logs })
    }
}
