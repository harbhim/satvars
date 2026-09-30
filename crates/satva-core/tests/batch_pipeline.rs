use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use anyhow::Result;
use satva_core::{
    BatchFilter, BatchPipeline, BatchSink, ErrorPolicy, PipelineOptions, RecordSinkAdapter,
    RecordSourceAdapter, SelectFieldsBatch, SetFieldBatch, Sink, Source,
};
use satva_expr::{field, lit};
use satva_types::{Record, Value};

struct VecSource {
    records: Vec<Record>,
}

impl Source for VecSource {
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>> {
        let records = self.records.clone();
        Ok(Box::new(records.into_iter().map(Ok)))
    }
}

struct MemSink {
    records: Arc<std::sync::Mutex<Vec<Record>>>,
    finishes: Arc<AtomicUsize>,
}

impl Sink for MemSink {
    fn write(&mut self, record: &Record) -> Result<()> {
        self.records.lock().expect("sink lock").push(record.clone());
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.finishes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn sample_records() -> Vec<Record> {
    (0..5)
        .map(|id| {
            let mut record = Record::new();
            record.insert("id", Value::Int64(id));
            record.insert("active", Value::Boolean(id % 2 == 0));
            record.insert("salary", Value::Float64(10_000.0 * (id as f64 + 1.0)));
            record
        })
        .collect()
}

#[test]
fn packs_row_source_and_writes_row_sink() {
    let source = VecSource {
        records: sample_records(),
    };
    let stored = Arc::new(std::sync::Mutex::new(Vec::new()));
    let finishes = Arc::new(AtomicUsize::new(0));
    let sink = MemSink {
        records: Arc::clone(&stored),
        finishes: Arc::clone(&finishes),
    };

    let mut pipeline = BatchPipeline::new(Box::new(RecordSourceAdapter::new(&source, 2).unwrap()));
    pipeline.add_stage(BatchFilter::new(field("active").equal_to(lit(true))));
    pipeline.add_stage(SetFieldBatch::new("bonus", field("salary").times(lit(0.1))));
    pipeline.add_stage(SelectFieldsBatch::new(["id", "bonus"]));
    pipeline.set_sink(Box::new(RecordSinkAdapter::new(Box::new(sink))));

    let result = pipeline
        .run(PipelineOptions::default().with_error_policy(ErrorPolicy::StopOnError))
        .unwrap();

    assert_eq!(result.summary.processed, 5);
    assert_eq!(result.summary.skipped, 2);
    assert_eq!(result.summary.succeeded, 3);
    assert_eq!(finishes.load(Ordering::SeqCst), 1);
    let written = stored.lock().expect("sink lock");
    assert_eq!(written.len(), 3);
    assert_eq!(written[0].get("bonus"), Some(&Value::Float64(1_000.0)));
    assert!(written[0].get("salary").is_none());
    assert_eq!(written[2].get("id"), Some(&Value::Int64(4)));
}

struct FailingBatchSink {
    writes: usize,
}

impl BatchSink for FailingBatchSink {
    fn write_batch(&mut self, _batch: &satva_arrow::RecordBatch) -> Result<()> {
        self.writes += 1;
        anyhow::bail!("disk full");
    }
}

#[test]
fn sink_failure_stops_and_still_finishes() {
    let records = sample_records();
    let source = VecSource { records };
    let adapter = RecordSourceAdapter::new(&source, 2).unwrap();
    let mut pipeline = BatchPipeline::new(Box::new(adapter));
    pipeline.set_sink(Box::new(FailingBatchSink { writes: 0 }));
    let error = pipeline
        .run(PipelineOptions::default().with_error_policy(ErrorPolicy::StopOnError))
        .unwrap_err();
    assert!(format!("{error:#}").contains("disk full"));
}
