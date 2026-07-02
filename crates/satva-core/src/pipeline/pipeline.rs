use crate::Sink;
use crate::source::Source;
use anyhow::Result;

use super::{
    PipelineLog, PipelineOptions, PipelineRunResult, PipelineStage, PipelineSummary, StageContext,
    StageResult,
};

enum RecordOutcome {
    Succeeded,
    Skipped,
    Failed,
}

pub struct Pipeline {
    source: Box<dyn Source>,
    stages: Vec<Box<dyn PipelineStage>>,
    sink: Option<Box<dyn Sink>>,
}

impl Pipeline {
    pub fn new(source: Box<dyn Source>) -> Self {
        Self {
            source,
            stages: Vec::new(),
            sink: None,
        }
    }

    pub fn add_stage(&mut self, stage: Box<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    pub fn set_sink(&mut self, sink: Box<dyn Sink>) {
        self.sink = Some(sink);
    }

    pub fn run(&mut self, options: PipelineOptions) -> Result<PipelineRunResult> {
        let records_iter = self.source.read()?;

        let mut summary = PipelineSummary::default();
        let mut logs = Vec::new();

        for (index, record_res) in records_iter.enumerate() {
            let record_index = index + 1;
            let mut record = record_res?;
            summary.processed += 1;

            let mut outcome = RecordOutcome::Succeeded;

            let stage_context = StageContext {
                record_index,
            };

            for stage in &self.stages {
                match stage.execute(&mut record, &stage_context) {
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
                            logs.push(PipelineLog::StageFailed {
                                record_index,
                                error,
                            });
                        }

                        break;
                    }
                }
            }

            match outcome {
                RecordOutcome::Succeeded => {
                    if let Some(sink) = self.sink.as_mut() {
                        match sink.write(&record) {
                            Ok(()) => summary.succeeded += 1,
                            Err(error) => {
                                summary.failed += 1;

                                if options.collect_logs {
                                    logs.push(PipelineLog::SinkFailed {
                                        record_index,
                                        message: error.to_string(),
                                    });
                                }
                            }
                        }
                    } else {
                        summary.succeeded += 1;
                    }
                }
                RecordOutcome::Skipped => summary.skipped += 1,
                RecordOutcome::Failed => summary.failed += 1,
            }
        }

        Ok(PipelineRunResult { summary, logs })
    }
}
