use satva_core::{
    record::Record, SchemaValidation, DataType, Field, Schema, Value,
    PipelineStage, StageContext, StageResult,
};

#[test]
fn test_schema_validation_success_and_coercion() {
    let schema = Schema::new(vec![
        Field::new("name", DataType::String, false),
        Field::new("age", DataType::Int64, false),
        Field::new("salary", DataType::Float64, true),
        Field::new("active", DataType::Boolean, true),
    ]);

    let validation = SchemaValidation::new(schema);
    let ctx = StageContext { record_index: 1 };

    // Record with string representation of numeric/boolean types
    let mut record = Record::new();
    record.insert("name", Value::string("Alice"));
    record.insert("age", Value::string("30"));
    record.insert("salary", Value::string("75000.50"));
    record.insert("active", Value::string("true"));

    let result = validation.execute(&mut record, &ctx);
    assert!(matches!(result, StageResult::Continue));

    assert_eq!(record.get("name").unwrap(), &Value::String("Alice".to_string()));
    assert_eq!(record.get("age").unwrap(), &Value::Int64(30));
    assert_eq!(record.get("salary").unwrap(), &Value::Float64(75000.50));
    assert_eq!(record.get("active").unwrap(), &Value::Boolean(true));
}

#[test]
fn test_schema_validation_nullable() {
    let schema = Schema::new(vec![
        Field::new("name", DataType::String, false),
        Field::new("age", DataType::Int64, true),
    ]);

    let validation = SchemaValidation::new(schema);
    let ctx = StageContext { record_index: 1 };

    let mut record = Record::new();
    record.insert("name", Value::string("Alice"));
    // "age" is missing

    let result = validation.execute(&mut record, &ctx);
    assert!(matches!(result, StageResult::Continue));
    assert_eq!(record.get("age").unwrap(), &Value::Null);
}

#[test]
fn test_schema_validation_missing_required() {
    let schema = Schema::new(vec![
        Field::new("name", DataType::String, false),
        Field::new("age", DataType::Int64, false),
    ]);

    let validation = SchemaValidation::new(schema);
    let ctx = StageContext { record_index: 1 };

    let mut record = Record::new();
    record.insert("name", Value::string("Alice"));
    // "age" is missing and not nullable

    let result = validation.execute(&mut record, &ctx);
    assert!(matches!(result, StageResult::Fail { .. }));
}

#[test]
fn test_schema_validation_invalid_type() {
    let schema = Schema::new(vec![
        Field::new("name", DataType::String, false),
        Field::new("age", DataType::Int64, false),
    ]);

    let validation = SchemaValidation::new(schema);
    let ctx = StageContext { record_index: 1 };

    let mut record = Record::new();
    record.insert("name", Value::string("Alice"));
    record.insert("age", Value::string("not_a_number"));

    let result = validation.execute(&mut record, &ctx);
    assert!(matches!(result, StageResult::Fail { .. }));
}
