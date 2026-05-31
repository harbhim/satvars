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
        let records = self.source.read()?;

        let mut summary = PipelineSummary::default();
        let mut logs = Vec::new();

        for record in records {
            summary.processed += 1;

            let mut current_record = record;
            let mut record_completed = true;

            for stage in &self.stages {
                match stage.execute(current_record) {
                    StageResult::Continue(r) => {
                        current_record = r;
                    }

                    StageResult::Skip { reason, .. } => {
                        summary.skipped += 1;
                        record_completed = false;

                        if options.collect_logs {
                            logs.push(PipelineLog::Skipped {
                                stage: stage.name(),
                                reason,
                            });
                        }

                        break;
                    }

                    StageResult::Fail { error, .. } => {
                        summary.failed += 1;
                        record_completed = false;

                        if options.collect_logs {
                            logs.push(PipelineLog::Failed { error });
                        }

                        break;
                    }
                }
            }

            if record_completed {
                summary.succeeded += 1;
            }
        }

        Ok(PipelineRunResult { summary, logs })
    }
}
