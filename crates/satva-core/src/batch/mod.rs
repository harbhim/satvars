//! Batch sources, sinks, and vectorized stages.
//!
//! Row [`crate::Source`] and [`crate::Sink`] values can be adapted at the boundary.
//! The batch pipeline then filters, projects, and computes fields on columns.

mod pipeline;
mod stages;

use std::sync::Arc;

use anyhow::Result;
use satva_arrow::{DEFAULT_BATCH_SIZE, RecordBatch};
use satva_types::Record;

use crate::{Sink, Source};

pub use pipeline::{AppliedBatch, BatchPipeline, apply_stages};
pub use stages::{BatchFilter, SelectFieldsBatch, SetFieldBatch};

pub use satva_arrow::DEFAULT_BATCH_SIZE as BATCH_SIZE;

/// Pulls the next columnar batch. `Ok(None)` ends the source.
pub trait BatchSource {
    fn read_batch(&mut self) -> Result<Option<RecordBatch>>;
}

/// Writes columnar batches. `finish` flushes buffered output.
pub trait BatchSink {
    fn write_batch(&mut self, batch: &RecordBatch) -> Result<()>;

    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}

/// One vectorized stage. Implementations must be safe to run on many batches at once.
pub trait BatchStage: Send + Sync {
    fn apply(&self, batch: RecordBatch) -> Result<AppliedBatch>;
}

/// Yields batches that were already built. Cloning the source clones column handles.
pub struct MemoryBatchSource {
    batches: Vec<RecordBatch>,
    index: usize,
}

impl MemoryBatchSource {
    pub fn new(batches: Vec<RecordBatch>) -> Self {
        Self { batches, index: 0 }
    }
}

impl BatchSource for MemoryBatchSource {
    fn read_batch(&mut self) -> Result<Option<RecordBatch>> {
        let batch = self.batches.get(self.index).cloned();
        if batch.is_some() {
            self.index += 1;
        }
        Ok(batch)
    }
}

/// Packs a row source into batches of `batch_size` rows.
pub struct RecordSourceAdapter {
    records: Box<dyn Iterator<Item = Result<Record>>>,
    batch_size: usize,
}

impl RecordSourceAdapter {
    pub fn new(source: &dyn Source, batch_size: usize) -> Result<Self> {
        Ok(Self {
            records: source.read()?,
            batch_size: batch_size.max(1),
        })
    }

    pub fn with_default_batch_size(source: &dyn Source) -> Result<Self> {
        Self::new(source, DEFAULT_BATCH_SIZE)
    }
}

impl BatchSource for RecordSourceAdapter {
    fn read_batch(&mut self) -> Result<Option<RecordBatch>> {
        let mut records = Vec::with_capacity(self.batch_size);
        for item in self.records.by_ref().take(self.batch_size) {
            records.push(item?);
        }
        if records.is_empty() {
            Ok(None)
        } else {
            Ok(Some(RecordBatch::from_records(&records)?))
        }
    }
}

/// Writes each batch back through a row sink.
pub struct RecordSinkAdapter {
    sink: Box<dyn Sink>,
}

impl RecordSinkAdapter {
    pub fn new(sink: Box<dyn Sink>) -> Self {
        Self { sink }
    }
}

impl BatchSink for RecordSinkAdapter {
    fn write_batch(&mut self, batch: &RecordBatch) -> Result<()> {
        for record in batch.to_records() {
            self.sink.write(&record)?;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.sink.finish()
    }
}

/// Shared stage list used by sequential and parallel runners.
pub type SharedStage = Arc<dyn BatchStage>;
