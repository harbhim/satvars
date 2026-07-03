use crate::source::Source;
use crate::{Record, Sink};
use anyhow::Result;

use super::{
    PipelineLog, PipelineOptions, PipelineRunResult, PipelineStage, PipelineSummary, StageContext,
    StageError, StageResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordOutcome {
    Succeeded,
    Skipped,
    Failed,
}

struct RecordExecutionResult {
    outcome: RecordOutcome,
}

impl RecordExecutionResult {
    fn new(outcome: RecordOutcome) -> Self {
        Self { outcome }
    }
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
        let records = self.source.read()?;

        let mut summary = PipelineSummary::default();
        let mut logs = Vec::new();

        for (index, record_result) in records.enumerate() {
            let record = record_result?;

            summary.processed += 1;

            let result = self.process_record(record, index + 1, &options, &mut logs);

            match result.outcome {
                RecordOutcome::Succeeded => summary.succeeded += 1,
                RecordOutcome::Skipped => summary.skipped += 1,
                RecordOutcome::Failed => summary.failed += 1,
            }
        }

        Ok(PipelineRunResult { summary, logs })
    }

    fn process_record(
        &mut self,
        mut record: Record,
        record_index: usize,
        options: &PipelineOptions,
        logs: &mut Vec<PipelineLog>,
    ) -> RecordExecutionResult {
        match self.execute_stages(&mut record, &StageContext { record_index }, options, logs) {
            result if result.outcome == RecordOutcome::Succeeded => {
                self.write_sink(&record, record_index, options, logs)
            }
            result => result,
        }
    }

    fn execute_stages(
        &self,
        record: &mut Record,
        context: &StageContext,
        options: &PipelineOptions,
        logs: &mut Vec<PipelineLog>,
    ) -> RecordExecutionResult {
        for stage in &self.stages {
            match stage.execute(record, context) {
                StageResult::Continue => {}

                StageResult::Skip { reason } => {
                    Self::log_stage_skip(options, logs, context.record_index, stage.name(), reason);

                    return RecordExecutionResult::new(RecordOutcome::Skipped);
                }

                StageResult::Fail { error } => {
                    Self::log_stage_failure(options, logs, context.record_index, error);

                    return RecordExecutionResult::new(RecordOutcome::Failed);
                }
            }
        }

        RecordExecutionResult::new(RecordOutcome::Succeeded)
    }

    fn write_sink(
        &mut self,
        record: &Record,
        record_index: usize,
        options: &PipelineOptions,
        logs: &mut Vec<PipelineLog>,
    ) -> RecordExecutionResult {
        if let Some(sink) = self.sink.as_mut() {
            match sink.write(record) {
                Ok(()) => RecordExecutionResult::new(RecordOutcome::Succeeded),

                Err(error) => {
                    Self::log_sink_failure(options, logs, record_index, error.to_string());

                    RecordExecutionResult::new(RecordOutcome::Failed)
                }
            }
        } else {
            RecordExecutionResult::new(RecordOutcome::Succeeded)
        }
    }

    fn log_stage_skip(
        options: &PipelineOptions,
        logs: &mut Vec<PipelineLog>,
        record_index: usize,
        stage: &'static str,
        reason: String,
    ) {
        if options.collect_logs {
            logs.push(PipelineLog::Skipped {
                record_index,
                stage,
                reason,
            });
        }
    }

    fn log_stage_failure(
        options: &PipelineOptions,
        logs: &mut Vec<PipelineLog>,
        record_index: usize,
        error: StageError,
    ) {
        if options.collect_logs {
            logs.push(PipelineLog::StageFailed {
                record_index,
                error,
            });
        }
    }

    fn log_sink_failure(
        options: &PipelineOptions,
        logs: &mut Vec<PipelineLog>,
        record_index: usize,
        message: String,
    ) {
        if options.collect_logs {
            logs.push(PipelineLog::SinkFailed {
                record_index,
                message,
            });
        }
    }
}
