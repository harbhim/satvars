use crate::source::Source;
use anyhow::Result;

use super::{
    PipelineLog, PipelineOptions, PipelineRunResult, PipelineStage, PipelineSummary, StageResult,
};

enum RecordOutcome {
    Succeeded,
    Skipped,
    Failed,
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

    pub fn run(&self, options: PipelineOptions) -> Result<PipelineRunResult> {
        let records = self.source.read()?;

        let mut summary = PipelineSummary::default();
        let mut logs = Vec::new();

        for (index, mut record) in records.into_iter().enumerate() {
            let record_index = index + 1;
            summary.processed += 1;

            let mut outcome = RecordOutcome::Succeeded;

            for stage in &self.stages {
                match stage.execute(&mut record) {
                    StageResult::Continue => {}

                    StageResult::Skip { reason } => {
                        outcome = RecordOutcome::Skipped;

                        if options.collect_logs {
                            logs.push(PipelineLog::Skipped {
                                record_index,
                                stage: stage.name(),
                                reason,
                            });
                        }

                        break;
                    }

                    StageResult::Fail { error } => {
                        outcome = RecordOutcome::Failed;

                        if options.collect_logs {
                            logs.push(PipelineLog::Failed {
                                record_index,
                                error,
                            });
                        }

                        break;
                    }
                }
            }

            match outcome {
                RecordOutcome::Succeeded => summary.succeeded += 1,
                RecordOutcome::Skipped => summary.skipped += 1,
                RecordOutcome::Failed => summary.failed += 1,
            }
        }

        Ok(PipelineRunResult { summary, logs })
    }
}
