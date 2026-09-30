use std::sync::{Arc, Mutex};

use anyhow::Result;
use satva_arrow::{Column, ColumnArray, RecordBatch};
use satva_core::{
    BatchFilter, BatchPipeline, BatchSink, MemoryBatchSource, PipelineOptions, SetFieldBatch,
};
use satva_execution::ParallelBatchPipeline;
use satva_expr::{field, lit};

struct IdSink(Arc<Mutex<Vec<i64>>>);

impl BatchSink for IdSink {
    fn write_batch(&mut self, batch: &RecordBatch) -> Result<()> {
        let Some(column) = batch.column("id") else {
            return Ok(());
        };
        let ColumnArray::Int64 { values, .. } = column.data().as_ref() else {
            return Ok(());
        };
        self.0.lock().expect("id lock").extend_from_slice(values);
        Ok(())
    }
}

fn batches() -> Vec<RecordBatch> {
    let batch = RecordBatch::try_new(vec![
        Column::int64("id", (0..100).map(i64::from).collect()),
        Column::boolean("active", (0..100).map(|id| id % 2 == 0).collect()),
        Column::float64(
            "salary",
            (0..100).map(|id| 1_000.0 * f64::from(id)).collect(),
        ),
    ])
    .unwrap();
    batch.split(10)
}

fn add_stages(sequential: &mut BatchPipeline, parallel: &mut ParallelBatchPipeline) {
    let filter = field("active").equal_to(lit(true));
    let bonus = field("salary").times(lit(0.25));
    sequential.add_stage(BatchFilter::new(filter.clone()));
    sequential.add_stage(SetFieldBatch::new("bonus", bonus.clone()));
    parallel.add_stage(BatchFilter::new(filter));
    parallel.add_stage(SetFieldBatch::new("bonus", bonus));
}

#[test]
fn parallel_matches_sequential_order_and_counts() {
    let mut sequential = BatchPipeline::new(Box::new(MemoryBatchSource::new(batches())));
    let mut parallel = ParallelBatchPipeline::new(Box::new(MemoryBatchSource::new(batches())));
    add_stages(&mut sequential, &mut parallel);
    let sequential_ids = Arc::new(Mutex::new(Vec::new()));
    let parallel_ids = Arc::new(Mutex::new(Vec::new()));
    sequential.set_sink(Box::new(IdSink(Arc::clone(&sequential_ids))));
    parallel.set_sink(Box::new(IdSink(Arc::clone(&parallel_ids))));

    let options = PipelineOptions::without_logs();
    let sequential = sequential.run(options).unwrap();
    let parallel = parallel.run(options).unwrap();

    assert_eq!(parallel.summary.processed, sequential.summary.processed);
    assert_eq!(parallel.summary.skipped, sequential.summary.skipped);
    assert_eq!(parallel.summary.succeeded, sequential.summary.succeeded);
    assert_eq!(parallel.summary.processed, 100);
    assert_eq!(parallel.summary.succeeded, 50);
    assert_eq!(
        parallel_ids.lock().expect("id lock").as_slice(),
        sequential_ids.lock().expect("id lock").as_slice()
    );
}
