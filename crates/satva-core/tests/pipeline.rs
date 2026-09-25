use anyhow::{Result, anyhow};
use satva_core::sink::Sink;
use satva_core::source::Source;
use satva_core::{
    Pipeline, PipelineLog, PipelineOptions, PipelineStage, StageContext, StageError, StageResult,
};
use satva_types::{Record, Value};
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
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>> {
        let it = self.records.clone().into_iter().map(Ok);
        Ok(Box::new(it))
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
        ..PipelineOptions::default()
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

struct FinishingSink {
    calls: Arc<AtomicUsize>,
    fail_write: bool,
    fail_finish: bool,
}

impl Sink for FinishingSink {
    fn write(&mut self, _: &Record) -> Result<()> {
        if self.fail_write {
            Err(anyhow!("write failed"))
        } else {
            Ok(())
        }
    }
    fn finish(&mut self) -> Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_finish {
            Err(anyhow!("flush failed"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn strict_errors_finish_sink_and_preserve_both_errors_without_logs() {
    for fail_finish in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let (stage, stage_calls) = TestStage::new(TestStageBehavior::Continue);
        let mut pipeline = Pipeline::builder()
            .source(TestSource::new(vec![record_with_id(1), record_with_id(2)]))
            .stage(stage)
            .sink(FinishingSink {
                calls: calls.clone(),
                fail_write: true,
                fail_finish,
            })
            .build()
            .unwrap();
        let error = pipeline
            .run(
                PipelineOptions::without_logs()
                    .with_error_policy(satva_core::ErrorPolicy::StopOnError),
            )
            .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("write failed"));
        assert!(message.contains("Record 1 failed"));
        assert_eq!(message.contains("flush failed"), fail_finish);
        assert_eq!(stage_calls.load(Ordering::SeqCst), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn finish_errors_fail_even_empty_runs_and_continue_policy() {
    for records in [vec![], vec![record_with_id(1)]] {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut pipeline = Pipeline::builder()
            .source(TestSource::new(records))
            .sink(FinishingSink {
                calls: calls.clone(),
                fail_write: false,
                fail_finish: true,
            })
            .build()
            .unwrap();
        let error = pipeline.run(PipelineOptions::default()).unwrap_err();
        assert!(format!("{error:#}").contains("flush failed"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn strict_stage_failure_stops_without_retained_logs() {
    let (stage, calls) = TestStage::new(TestStageBehavior::Fail("invalid data"));
    let mut pipeline = Pipeline::builder()
        .source(TestSource::new(vec![record_with_id(1), record_with_id(2)]))
        .stage(stage)
        .build()
        .unwrap();
    let error = pipeline
        .run(
            PipelineOptions::default()
                .with_log_limit(Some(0))
                .with_error_policy(satva_core::ErrorPolicy::StopOnError),
        )
        .unwrap_err();
    assert!(format!("{error:#}").contains("invalid data"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn logs_are_bounded_but_summary_counts_every_record() {
    for (limit, expected) in [
        (Some(0), 0),
        (Some(2), 2),
        (Some(1_000), 1_000),
        (None, 1_005),
    ] {
        for behavior in [
            TestStageBehavior::Skip("skip"),
            TestStageBehavior::Fail("fail"),
        ] {
            let (stage, _) = TestStage::new(behavior);
            let mut pipeline = Pipeline::builder()
                .source(TestSource::new(vec![record_with_id(1); 1_005]))
                .stage(stage)
                .build()
                .unwrap();
            let result = pipeline
                .run(PipelineOptions::default().with_log_limit(limit))
                .unwrap();
            assert_eq!(result.logs.len(), expected);
            assert_eq!(result.summary.processed, 1_005);
            assert_eq!(result.summary.failed + result.summary.skipped, 1_005);
        }
    }
}

struct BrokenSource {
    fail_open: bool,
}
impl Source for BrokenSource {
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>> {
        if self.fail_open {
            return Err(anyhow!("source failed"));
        }
        Ok(Box::new(
            vec![Ok(record_with_id(1)), Err(anyhow!("source failed"))].into_iter(),
        ))
    }
}

#[test]
fn source_errors_still_finish_sink() {
    for fail_open in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut pipeline = Pipeline::builder()
            .source(BrokenSource { fail_open })
            .sink(FinishingSink {
                calls: calls.clone(),
                fail_write: false,
                fail_finish: false,
            })
            .build()
            .unwrap();
        assert!(pipeline.run(PipelineOptions::default()).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn continue_policy_counts_all_sink_failures_and_finishes() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut pipeline = Pipeline::builder()
        .source(TestSource::new(vec![record_with_id(1); 3]))
        .sink(FinishingSink {
            calls: calls.clone(),
            fail_write: true,
            fail_finish: false,
        })
        .build()
        .unwrap();
    let result = pipeline
        .run(PipelineOptions::default().with_log_limit(Some(1)))
        .unwrap();
    assert_eq!(result.summary.processed, 3);
    assert_eq!(result.summary.failed, 3);
    assert_eq!(result.summary.succeeded, 0);
    assert_eq!(result.logs.len(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
