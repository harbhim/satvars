use satva_types::{DataType, Field, Record, Schema, Value};

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

    let field_names = schema.fields().iter().map(Field::name).collect::<Vec<_>>();

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

#[test]
fn schema_can_be_inferred_from_records() {
    let mut r1 = Record::new();
    r1.insert("id", Value::string("1"));
    r1.insert("name", Value::string("Alice"));
    r1.insert("active", Value::string("true"));
    r1.insert("salary", Value::string("50000.50"));

    let mut r2 = Record::new();
    r2.insert("id", Value::string("2"));
    r2.insert("name", Value::string("Bob"));
    r2.insert("active", Value::string("false"));
    // salary is missing in r2 (which should make it nullable)

    let schema = Schema::infer(&[r1, r2]);

    assert_eq!(schema.fields().len(), 4);
    assert_eq!(schema.fields()[0].name(), "active");
    assert_eq!(schema.fields()[0].data_type(), DataType::Boolean);
    assert!(!schema.fields()[0].is_nullable());

    assert_eq!(schema.fields()[1].name(), "id");
    assert_eq!(schema.fields()[1].data_type(), DataType::Int64);
    assert!(!schema.fields()[1].is_nullable());

    assert_eq!(schema.fields()[2].name(), "name");
    assert_eq!(schema.fields()[2].data_type(), DataType::String);
    assert!(!schema.fields()[2].is_nullable());

    assert_eq!(schema.fields()[3].name(), "salary");
    assert_eq!(schema.fields()[3].data_type(), DataType::Float64);
    assert!(schema.fields()[3].is_nullable());
}
