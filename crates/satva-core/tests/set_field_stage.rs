use satva_core::{PipelineStage, SetFieldStage, StageContext, StageResult};
use satva_expr::{field, lit};
use satva_types::{Record, Value};

fn context() -> StageContext {
    StageContext { record_index: 0 }
}

#[test]
fn creates_new_field() {
    let mut record = Record::new();
    record.insert("salary", 100.into());
    record.insert("bonus", 20.into());

    let stage = SetFieldStage::new("total_salary", field("salary").plus(field("bonus")));

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));
    assert_eq!(record.get("total_salary"), Some(&Value::Int64(120)));
}

#[test]
fn overwrites_existing_field() {
    let mut record = Record::new();
    record.insert("salary", 100.into());

    let stage = SetFieldStage::new("salary", field("salary").plus(lit(50)));

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));
    assert_eq!(record.get("salary"), Some(&Value::Int64(150)));
}

#[test]
fn copies_field() {
    let mut record = Record::new();
    record.insert("first_name", "Hardik".into());

    let stage = SetFieldStage::new("name", field("first_name"));

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));
    assert_eq!(
        record.get("name"),
        Some(&Value::String("Hardik".to_string()))
    );
}

#[test]
fn stores_literal() {
    let mut record = Record::new();

    let stage = SetFieldStage::new("country", lit("India"));

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));
    assert_eq!(
        record.get("country"),
        Some(&Value::String("India".to_string()))
    );
}

#[test]
fn fails_when_expression_cannot_be_evaluated() {
    let mut record = Record::new();

    let stage = SetFieldStage::new("salary", field("missing"));

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Fail { .. }));
}
