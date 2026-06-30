use anyhow::{Result, anyhow};
use satva_core::record::Record;
use satva_core::sink::Sink;
use satva_core::source::Source;
use satva_core::value::Value;
use satva_core::{
    Pipeline, PipelineLog, PipelineOptions, PipelineStage, StageContext, StageError, StageResult,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

struct TestSource {
    records: Vec<Record>,
}

impl TestSource {
    fn new(records: Vec<Record>) -> Self {
        Self { records }
    }
}

impl Source for TestSource {
    fn read(&self) -> Result<Vec<Record>> {
        Ok(self.records.clone())
    }
}

enum TestStageBehavior {
    Continue,
    Skip(&'static str),
    Fail(&'static str),
}

struct TestStage {
    behavior: TestStageBehavior,
    calls: Arc<AtomicUsize>,
}

impl TestStage {
    fn new(behavior: TestStageBehavior) -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));

        (
            Self {
                behavior,
                calls: Arc::clone(&calls),
            },
            calls,
        )
    }
}

impl PipelineStage for TestStage {
    fn name(&self) -> &'static str {
        "TestStage"
    }

    fn execute(&self, _: &mut Record, _: &StageContext) -> StageResult {
        self.calls.fetch_add(1, Ordering::SeqCst);

        match self.behavior {
            TestStageBehavior::Continue => StageResult::Continue,
            TestStageBehavior::Skip(reason) => StageResult::Skip {
                reason: reason.to_string(),
            },
            TestStageBehavior::Fail(message) => StageResult::Fail {
                error: StageError::execution(self.name(), message),
            },
        }
    }
}

struct RecordingSink {
    records: Arc<Mutex<Vec<Record>>>,
}

impl RecordingSink {
    fn new() -> (Self, Arc<Mutex<Vec<Record>>>) {
        let records = Arc::new(Mutex::new(Vec::new()));

        (
            Self {
                records: Arc::clone(&records),
            },
            records,
        )
    }
}

impl Sink for RecordingSink {
    fn write(&mut self, record: &Record) -> Result<()> {
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }
}

struct FailFirstSink {
    calls: usize,
}

impl Sink for FailFirstSink {
    fn write(&mut self, _: &Record) -> Result<()> {
        self.calls += 1;

        if self.calls == 1 {
            return Err(anyhow!("sink is unavailable"));
        }

        Ok(())
    }
}

fn record_with_id(id: i64) -> Record {
    let mut record = Record::new();
    record.insert("id", Value::Int64(id));
    record
}

#[test]
fn record_with_all_continue_stages_succeeds() -> Result<()> {
    let source = Box::new(TestSource::new(vec![record_with_id(1)]));
    let (stage, _) = TestStage::new(TestStageBehavior::Continue);
    let mut pipeline = Pipeline::new(source);

    pipeline.add_stage(Box::new(stage));

    let result = pipeline.run(PipelineOptions::default())?;

    assert_eq!(result.summary.processed, 1);
    assert_eq!(result.summary.succeeded, 1);
    assert_eq!(result.summary.skipped, 0);
    assert_eq!(result.summary.failed, 0);
    assert!(result.logs.is_empty());

    Ok(())
}

#[test]
fn skip_increments_skipped_and_stops_later_stages() -> Result<()> {
    let source = Box::new(TestSource::new(vec![record_with_id(1)]));
    let (skip_stage, _) = TestStage::new(TestStageBehavior::Skip("not needed"));
    let (later_stage, later_calls) = TestStage::new(TestStageBehavior::Continue);
    let mut pipeline = Pipeline::new(source);

    pipeline.add_stage(Box::new(skip_stage));
    pipeline.add_stage(Box::new(later_stage));

    let result = pipeline.run(PipelineOptions::default())?;

    assert_eq!(result.summary.processed, 1);
    assert_eq!(result.summary.succeeded, 0);
    assert_eq!(result.summary.skipped, 1);
    assert_eq!(result.summary.failed, 0);
    assert_eq!(later_calls.load(Ordering::SeqCst), 0);

    match &result.logs[0] {
        PipelineLog::Skipped {
            record_index,
            stage,
            reason,
        } => {
            assert_eq!(*record_index, 1);
            assert_eq!(*stage, "TestStage");
            assert_eq!(reason, "not needed");
        }
        _ => panic!("expected skipped log"),
    }

    Ok(())
}

#[test]
fn fail_increments_failed_and_stops_later_stages() -> Result<()> {
    let source = Box::new(TestSource::new(vec![record_with_id(1)]));
    let (fail_stage, _) = TestStage::new(TestStageBehavior::Fail("bad record"));
    let (later_stage, later_calls) = TestStage::new(TestStageBehavior::Continue);
    let mut pipeline = Pipeline::new(source);

    pipeline.add_stage(Box::new(fail_stage));
    pipeline.add_stage(Box::new(later_stage));

    let result = pipeline.run(PipelineOptions::default())?;

    assert_eq!(result.summary.processed, 1);
    assert_eq!(result.summary.succeeded, 0);
    assert_eq!(result.summary.skipped, 0);
    assert_eq!(result.summary.failed, 1);
    assert_eq!(later_calls.load(Ordering::SeqCst), 0);

    match &result.logs[0] {
        PipelineLog::StageFailed {
            record_index,
            error,
        } => {
            assert_eq!(*record_index, 1);

            match error {
                StageError::Execution { stage, message } => {
                    assert_eq!(*stage, "TestStage");
                    assert_eq!(message, "bad record");
                }
            }
        }
        _ => panic!("expected failed log"),
    }

    Ok(())
}

#[test]
fn collect_logs_false_returns_empty_logs() -> Result<()> {
    let source = Box::new(TestSource::new(vec![record_with_id(1)]));
    let (stage, _) = TestStage::new(TestStageBehavior::Fail("bad record"));
    let mut pipeline = Pipeline::new(source);

    pipeline.add_stage(Box::new(stage));

    let result = pipeline.run(PipelineOptions {
        collect_logs: false,
    })?;

    assert_eq!(result.summary.failed, 1);
    assert!(result.logs.is_empty());

    Ok(())
}

#[test]
fn attached_sink_receives_successful_records() -> Result<()> {
    let source = Box::new(TestSource::new(vec![record_with_id(1), record_with_id(2)]));
    let (sink, written_records) = RecordingSink::new();
    let mut pipeline = Pipeline::new(source);

    pipeline.set_sink(Box::new(sink));

    let result = pipeline.run(PipelineOptions::default())?;

    assert_eq!(result.summary.processed, 2);
    assert_eq!(result.summary.succeeded, 2);
    assert_eq!(written_records.lock().unwrap().len(), 2);

    Ok(())
}

#[test]
fn sink_failure_increments_failed_logs_error_and_continues() -> Result<()> {
    let source = Box::new(TestSource::new(vec![record_with_id(1), record_with_id(2)]));
    let mut pipeline = Pipeline::new(source);

    pipeline.set_sink(Box::new(FailFirstSink { calls: 0 }));

    let result = pipeline.run(PipelineOptions::default())?;

    assert_eq!(result.summary.processed, 2);
    assert_eq!(result.summary.succeeded, 1);
    assert_eq!(result.summary.skipped, 0);
    assert_eq!(result.summary.failed, 1);

    match &result.logs[0] {
        PipelineLog::SinkFailed {
            record_index,
            message,
        } => {
            assert_eq!(*record_index, 1);
            assert_eq!(message, "sink is unavailable");
        }
        _ => panic!("expected sink failed log"),
    }

    Ok(())
}
