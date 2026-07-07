use satva_core::{FilterStage, PipelineStage, StageContext, StageResult};
use satva_expr::{field, lit};
use satva_types::{Record, Value};

fn context() -> StageContext {
    StageContext { record_index: 0 }
}

#[test]
fn filter_continues_when_expression_is_true() {
    let mut record = Record::new();
    record.insert("age", 25.into());

    let stage = FilterStage::new(field("age").greater_than(lit(18)));

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));
}

#[test]
fn filter_skips_when_expression_is_false() {
    let mut record = Record::new();
    record.insert("age", 15.into());

    let stage = FilterStage::new(field("age").greater_than(lit(18)));

    let result = stage.execute(&mut record, &context());

    match result {
        StageResult::Skip { reason } => {
            assert_eq!(reason, "Record did not satisfy filter");
        }
        _ => panic!("Expected Skip"),
    }
}

#[test]
fn filter_uses_custom_skip_reason() {
    let mut record = Record::new();
    record.insert("age", 15.into());

    let stage = FilterStage::new(field("age").greater_than(lit(18)))
        .with_reason("Employee must be an adult");

    let result = stage.execute(&mut record, &context());

    match result {
        StageResult::Skip { reason } => {
            assert_eq!(reason, "Employee must be an adult");
        }
        _ => panic!("Expected Skip"),
    }
}

#[test]
fn filter_fails_when_expression_does_not_return_boolean() {
    let mut record = Record::new();
    record.insert("salary", 50000.into());

    let stage = FilterStage::new(field("salary"));

    let result = stage.execute(&mut record, &context());

    match result {
        StageResult::Fail { error } => {
            assert!(
                error.to_string().contains("expected Boolean"),
                "Unexpected error: {error}"
            );
        }
        _ => panic!("Expected Fail"),
    }
}

#[test]
fn filter_fails_when_field_is_missing() {
    let mut record = Record::new();

    let stage = FilterStage::new(field("salary").greater_than(lit(1000)));

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Fail { .. }));
}

#[test]
fn filter_does_not_modify_record() {
    let mut record = Record::new();
    record.insert("age", Value::Int64(25));

    let stage = FilterStage::new(field("age").greater_than(lit(18)));

    let _ = stage.execute(&mut record, &context());

    assert_eq!(record.get("age"), Some(&Value::Int64(25)));
}
