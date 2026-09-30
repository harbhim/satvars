use anyhow::{Context, Result};
use satva_arrow::RecordBatch;

use crate::{ErrorPolicy, PipelineLog, PipelineOptions, PipelineRunResult, PipelineSummary};

use super::{BatchSink, BatchSource, BatchStage, SharedStage};

/// Output of one batch stage, including rows removed by filters.
pub struct AppliedBatch {
    pub batch: RecordBatch,
    pub skipped: usize,
}

/// Runs vectorized stages over each batch, then writes surviving rows in order.
pub struct BatchPipeline {
    source: Box<dyn BatchSource>,
    stages: Vec<SharedStage>,
    sink: Option<Box<dyn BatchSink>>,
}

impl BatchPipeline {
    pub fn new(source: Box<dyn BatchSource>) -> Self {
        Self {
            source,
            stages: Vec::new(),
            sink: None,
        }
    }

    pub fn add_stage(&mut self, stage: impl BatchStage + 'static) {
        self.stages.push(std::sync::Arc::new(stage));
    }

    pub fn stages(&self) -> &[SharedStage] {
        &self.stages
    }

    pub fn set_sink(&mut self, sink: Box<dyn BatchSink>) {
        self.sink = Some(sink);
    }

    pub fn run(&mut self, options: PipelineOptions) -> Result<PipelineRunResult> {
        let result = self.execute(options);
        let finished = self
            .sink
            .as_mut()
            .map_or(Ok(()), |sink| sink.finish())
            .context("Failed to finish sink");
        match (result, finished) {
            (Ok(result), Ok(())) => Ok(result),
            (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(finish_error)) => Err(error.context(format!("{finish_error:#}"))),
        }
    }

    fn execute(&mut self, options: PipelineOptions) -> Result<PipelineRunResult> {
        let mut summary = PipelineSummary::default();
        let mut logs = Vec::new();
        let mut failure = None;
        while let Some(batch) = self.source.read_batch()? {
            summary.add_processed(batch.num_rows());
            let applied = apply_stages(batch, &self.stages)?;
            summary.add_skipped(applied.skipped);
            if applied.batch.num_rows() == 0 {
                continue;
            }
            if let Some(error) = write_batch(self.sink.as_mut(), &applied, &options, &mut logs) {
                summary.add_failed(applied.batch.num_rows());
                if options.error_policy == ErrorPolicy::StopOnError {
                    failure = Some(error.context(format!("Record {} failed", summary.processed)));
                    break;
                }
            } else {
                summary.add_succeeded(applied.batch.num_rows());
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(PipelineRunResult { summary, logs })
    }
}

pub fn apply_stages(batch: RecordBatch, stages: &[SharedStage]) -> Result<AppliedBatch> {
    let mut current = batch;
    let mut skipped = 0;
    for stage in stages {
        let applied = stage.apply(current)?;
        skipped += applied.skipped;
        current = applied.batch;
    }
    Ok(AppliedBatch {
        batch: current,
        skipped,
    })
}

pub(crate) fn write_batch(
    sink: Option<&mut Box<dyn BatchSink>>,
    applied: &AppliedBatch,
    options: &PipelineOptions,
    logs: &mut Vec<PipelineLog>,
) -> Option<anyhow::Error> {
    let sink = sink?;
    match sink.write_batch(&applied.batch) {
        Ok(()) => None,
        Err(error) => {
            if options.should_log(logs.len()) {
                logs.push(PipelineLog::SinkFailed {
                    record_index: applied.batch.num_rows(),
                    message: error.to_string(),
                });
            }
            Some(error)
        }
    }
}
