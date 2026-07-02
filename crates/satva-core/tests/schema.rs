use satva_core::{DataType, Field, Schema};

#[test]
fn schema_can_be_created_with_fields() {
    let schema = Schema::new(vec![
        Field::new("name", DataType::String, false),
        Field::new("age", DataType::Int64, true),
    ]);

    assert_eq!(schema.fields().len(), 2);
    assert_eq!(schema.fields()[0].name(), "name");
    assert_eq!(schema.fields()[0].data_type(), DataType::String);
    assert!(!schema.fields()[0].is_nullable());
    assert_eq!(schema.fields()[1].name(), "age");
    assert_eq!(schema.fields()[1].data_type(), DataType::Int64);
    assert!(schema.fields()[1].is_nullable());
}

#[test]
fn schema_preserves_field_order() {
    let schema = Schema::new(vec![
        Field::new("employee_id", DataType::Int64, false),
        Field::new("first_name", DataType::String, false),
        Field::new("salary", DataType::Float64, true),
    ]);

    let field_names = schema
        .fields()
        .iter()
        .map(satva_core::Field::name)
        .collect::<Vec<_>>();

    assert_eq!(field_names, vec!["employee_id", "first_name", "salary"]);
}

#[test]
fn schema_can_find_field_by_name() {
    let schema = Schema::new(vec![
        Field::new("active", DataType::Boolean, false),
        Field::new("department", DataType::String, true),
    ]);

    let field = schema.field("department").unwrap();

    assert_eq!(field.name(), "department");
    assert_eq!(field.data_type(), DataType::String);
    assert!(field.is_nullable());
    assert!(schema.field("missing").is_none());
}
