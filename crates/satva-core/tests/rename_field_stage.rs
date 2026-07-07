use satva_core::{PipelineStage, RenameFieldStage, StageContext, StageResult};
use satva_types::{Record, Value};

fn context() -> StageContext {
    StageContext { record_index: 0 }
}

#[test]
fn renames_existing_field() {
    let mut record = Record::new();
    record.insert("name", "Hardik".into());

    let stage = RenameFieldStage::new("name", "full_name");

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(
        record.get("full_name"),
        Some(&Value::String("Hardik".to_string()))
    );

    assert!(record.get("name").is_none());
}

#[test]
fn does_nothing_when_source_field_is_missing() {
    let mut record = Record::new();
    record.insert("age", 25.into());

    let stage = RenameFieldStage::new("name", "full_name");

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(record.get("age"), Some(&Value::Int64(25)));
    assert!(record.get("name").is_none());
    assert!(record.get("full_name").is_none());
}

#[test]
fn overwrites_destination_field_if_it_exists() {
    let mut record = Record::new();
    record.insert("first_name", "Hardik".into());
    record.insert("name", "Old Value".into());

    let stage = RenameFieldStage::new("first_name", "name");

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(
        record.get("name"),
        Some(&Value::String("Hardik".to_string()))
    );

    assert!(record.get("first_name").is_none());
}

#[test]
fn can_rename_numeric_fields() {
    let mut record = Record::new();
    record.insert("salary", 50000.into());

    let stage = RenameFieldStage::new("salary", "base_salary");

    let result = stage.execute(&mut record, &context());

    assert!(matches!(result, StageResult::Continue));

    assert_eq!(record.get("base_salary"), Some(&Value::Int64(50000)));

    assert!(record.get("salary").is_none());
}
