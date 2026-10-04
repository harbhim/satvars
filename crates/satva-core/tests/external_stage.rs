use std::sync::Arc;

use satva_core::{
    ExternalCompare, ExternalPolicy, ExternalStage, ExternalTable, PipelineStage, StageContext,
    StageResult,
};
use satva_types::{Record, Value};

fn context() -> StageContext {
    StageContext { record_index: 1 }
}

fn record(sku: &str, name: &str, price: i64) -> Record {
    let mut record = Record::new();
    record.insert("sku", Value::string(sku));
    record.insert("name", Value::string(name));
    record.insert("price", Value::Int64(price));
    record
}

fn stage(rows: Vec<Record>, on_differ: ExternalPolicy) -> ExternalStage {
    let table = ExternalTable::from_records(&["sku".to_string()], rows).expect("index");
    ExternalStage::new(
        "catalog",
        ExternalCompare {
            key: vec!["sku".to_string()],
            compare: Some(vec!["name".to_string(), "price".to_string()]),
            on_missing: ExternalPolicy::Continue,
            on_match: ExternalPolicy::Skip,
            on_differ,
        },
        Arc::new(table),
    )
    .expect("stage")
}

#[test]
fn external_stage_skips_matches_replaces_differences_and_keeps_new_rows() {
    let stage = stage(
        vec![record("same", "Same", 5), record("changed", "FromDb", 9)],
        ExternalPolicy::Replace,
    );
    let ctx = context();

    let mut fresh = record("new", "Widget", 10);
    assert!(matches!(
        stage.execute(&mut fresh, &ctx),
        StageResult::Continue
    ));
    assert_eq!(fresh.get("name").and_then(Value::as_string), Some("Widget"));

    let mut same = record("same", "Same", 5);
    match stage.execute(&mut same, &ctx) {
        StageResult::Skip { reason } => assert!(reason.contains("sku=same"), "{reason}"),
        other => panic!("expected skip, got {other:?}"),
    }

    let mut changed = record("changed", "Old", 1);
    assert!(matches!(
        stage.execute(&mut changed, &ctx),
        StageResult::Continue
    ));
    assert_eq!(
        changed.get("name").and_then(Value::as_string),
        Some("FromDb")
    );
    assert_eq!(changed.get("price").and_then(Value::as_i64), Some(9));
    assert_eq!(
        changed.get("sku").and_then(Value::as_string),
        Some("changed")
    );
}

#[test]
fn external_stage_can_fail_when_values_differ() {
    let stage = stage(vec![record("bad", "Bad", 4)], ExternalPolicy::Fail);
    let mut row = record("bad", "Bad", 3);
    match stage.execute(&mut row, &context()) {
        StageResult::Fail { error } => assert!(error.to_string().contains("differs")),
        other => panic!("expected fail, got {other:?}"),
    }
}

#[test]
fn external_stage_rejects_a_missing_key() {
    let stage = stage(vec![], ExternalPolicy::Replace);
    let mut row = Record::new();
    row.insert("name", Value::string("Widget"));
    match stage.execute(&mut row, &context()) {
        StageResult::Fail { error } => assert!(error.to_string().contains("sku")),
        other => panic!("expected fail, got {other:?}"),
    }
}

#[test]
fn external_table_rejects_duplicate_keys() {
    let error = ExternalTable::from_records(
        &["sku".to_string()],
        vec![record("same", "A", 1), record("same", "B", 2)],
    )
    .expect_err("duplicate");
    assert!(error.contains("duplicate"), "{error}");
}
