use satva_core::{PipelineStage, RemoveFieldStage, StageContext, StageResult};
use satva_types::{Record, Value};

fn context() -> StageContext {
    StageContext { record_index: 0 }
}

#[test]
fn removes_single_field() {
    let mut record = Record::new();
    record.insert("name", "Hardik".into());
    record.insert("age", 25.into());

    let stage = RemoveFieldStage::new(["age"]);

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(
        record.get("name"),
        Some(&Value::String("Hardik".to_string()))
    );
    assert!(record.get("age").is_none());
}

#[test]
fn removes_multiple_fields() {
    let mut record = Record::new();
    record.insert("name", "Hardik".into());
    record.insert("age", 25.into());
    record.insert("city", "Rajkot".into());

    let stage = RemoveFieldStage::new(["age", "city"]);

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert!(record.get("age").is_none());
    assert!(record.get("city").is_none());

    assert_eq!(
        record.get("name"),
        Some(&Value::String("Hardik".to_string()))
    );
}

#[test]
fn ignores_missing_fields() {
    let mut record = Record::new();
    record.insert("name", "Hardik".into());

    let stage = RemoveFieldStage::new(["salary", "department"]);

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(
        record.get("name"),
        Some(&Value::String("Hardik".to_string()))
    );
}

#[test]
fn removes_all_requested_fields() {
    let mut record = Record::new();
    record.insert("a", 1.into());
    record.insert("b", 2.into());
    record.insert("c", 3.into());

    let stage = RemoveFieldStage::new(["a", "b", "c"]);

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert!(record.get("a").is_none());
    assert!(record.get("b").is_none());
    assert!(record.get("c").is_none());
}

#[test]
fn removing_no_fields_does_nothing() {
    let mut record = Record::new();
    record.insert("name", "Hardik".into());

    let stage = RemoveFieldStage::new(Vec::<&str>::new());

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(
        record.get("name"),
        Some(&Value::String("Hardik".to_string()))
    );
}
