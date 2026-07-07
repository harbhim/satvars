use satva_core::{PipelineStage, SelectFieldsStage, StageContext, StageResult};
use satva_types::{Record, Value};

fn context() -> StageContext {
    StageContext { record_index: 0 }
}

#[test]
fn keeps_selected_fields() {
    let mut record = Record::new();
    record.insert("id", 1.into());
    record.insert("name", "Hardik".into());
    record.insert("salary", 50000.into());

    let stage = SelectFieldsStage::new(["id", "name"]);

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(record.get("id"), Some(&Value::Int64(1)));
    assert_eq!(
        record.get("name"),
        Some(&Value::String("Hardik".to_string()))
    );
    assert!(record.get("salary").is_none());
}

#[test]
fn ignores_missing_selected_fields() {
    let mut record = Record::new();
    record.insert("id", 1.into());

    let stage = SelectFieldsStage::new(["id", "name"]);

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(record.get("id"), Some(&Value::Int64(1)));
    assert!(record.get("name").is_none());
}

#[test]
fn removes_everything_when_selection_is_empty() {
    let mut record = Record::new();
    record.insert("id", 1.into());
    record.insert("name", "Hardik".into());

    let stage = SelectFieldsStage::new(Vec::<&str>::new());

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert!(record.fields.is_empty());
}

#[test]
fn keeps_all_requested_fields() {
    let mut record = Record::new();
    record.insert("a", 1.into());
    record.insert("b", 2.into());
    record.insert("c", 3.into());

    let stage = SelectFieldsStage::new(["a", "b", "c"]);

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(record.fields.len(), 3);
}

#[test]
fn preserves_values() {
    let mut record = Record::new();
    record.insert("active", true.into());
    record.insert("name", "Hardik".into());

    let stage = SelectFieldsStage::new(["active"]);

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(record.get("active"), Some(&Value::Boolean(true)));
}
