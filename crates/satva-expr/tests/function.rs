use anyhow::Result;
use satva_expr::{Evaluator, coalesce, concat, field, lit};
use satva_types::{Record, Value};

#[test]
fn upper_function() -> Result<()> {
    let mut record = Record::new();
    record.insert("name", "hardik".into());

    let value = Evaluator::evaluate(&field("name").upper(), &record)?;

    assert_eq!(value, Value::String("HARDIK".to_string()));

    Ok(())
}

#[test]
fn lower_function() -> Result<()> {
    let mut record = Record::new();
    record.insert("name", "HARDIK".into());

    let value = Evaluator::evaluate(&field("name").lower(), &record)?;

    assert_eq!(value, Value::String("hardik".to_string()));

    Ok(())
}

#[test]
fn trim_function() -> Result<()> {
    let mut record = Record::new();
    record.insert("name", "  Hardik  ".into());

    let value = Evaluator::evaluate(&field("name").trim(), &record)?;

    assert_eq!(value, Value::String("Hardik".to_string()));

    Ok(())
}

#[test]
fn length_function() -> Result<()> {
    let mut record = Record::new();
    record.insert("name", "Hardik".into());

    let value = Evaluator::evaluate(&field("name").length(), &record)?;

    assert_eq!(value, Value::Int64(6));

    Ok(())
}

#[test]
fn concat_function() -> Result<()> {
    let mut record = Record::new();
    record.insert("first_name", "Hardik".into());
    record.insert("last_name", "Bhimani".into());

    let expr = concat([field("first_name"), lit(" "), field("last_name")]);

    let value = Evaluator::evaluate(&expr, &record)?;

    assert_eq!(value, Value::String("Hardik Bhimani".to_string()));

    Ok(())
}

#[test]
fn concat_multiple_literals() -> Result<()> {
    let record = Record::new();

    let expr = concat([lit("Hello"), lit(", "), lit("World"), lit("!")]);

    let value = Evaluator::evaluate(&expr, &record)?;

    assert_eq!(value, Value::String("Hello, World!".to_string()));

    Ok(())
}

#[test]
fn upper_requires_string() {
    let mut record = Record::new();
    record.insert("age", 25.into());

    let result = Evaluator::evaluate(&field("age").upper(), &record);

    assert!(result.is_err());
}

#[test]
fn concat_requires_strings() {
    let mut record = Record::new();
    record.insert("name", "Hardik".into());
    record.insert("age", 25.into());

    let expr = concat([field("name"), field("age")]);

    let result = Evaluator::evaluate(&expr, &record);

    assert!(result.is_err());
}

#[test]
fn coalesce_returns_first_non_null() -> Result<()> {
    let record = Record::new();

    let expr = coalesce([
        lit(Value::Null),
        lit(Value::Null),
        lit("Engineering"),
        lit("HR"),
    ]);

    let value = Evaluator::evaluate(&expr, &record)?;

    assert_eq!(value, Value::String("Engineering".to_string()));

    Ok(())
}

#[test]
fn coalesce_returns_null_when_all_null() -> Result<()> {
    let record = Record::new();

    let expr = coalesce([lit(Value::Null), lit(Value::Null)]);

    let value = Evaluator::evaluate(&expr, &record)?;

    assert_eq!(value, Value::Null);

    Ok(())
}

#[test]
fn is_null_returns_true() -> Result<()> {
    let mut record = Record::new();
    record.insert("department", Value::Null);

    let value = Evaluator::evaluate(&field("department").is_null(), &record)?;

    assert_eq!(value, Value::Boolean(true));

    Ok(())
}

#[test]
fn is_not_null_returns_true() -> Result<()> {
    let mut record = Record::new();
    record.insert("department", "Engineering".into());

    let value = Evaluator::evaluate(&field("department").is_not_null(), &record)?;

    assert_eq!(value, Value::Boolean(true));

    Ok(())
}

#[test]
fn cast_int_from_string() -> Result<()> {
    let mut record = Record::new();
    record.insert("age", "42".into());

    let value = Evaluator::evaluate(&field("age").cast_int(), &record)?;

    assert_eq!(value, Value::Int64(42));

    Ok(())
}

#[test]
fn cast_int_from_float() -> Result<()> {
    let mut record = Record::new();
    record.insert("salary", Value::Float64(42.9));

    let value = Evaluator::evaluate(&field("salary").cast_int(), &record)?;

    assert_eq!(value, Value::Int64(42));

    Ok(())
}

#[test]
fn cast_float_from_string() -> Result<()> {
    let mut record = Record::new();
    record.insert("salary", "42.5".into());

    let value = Evaluator::evaluate(&field("salary").cast_float(), &record)?;

    assert_eq!(value, Value::Float64(42.5));

    Ok(())
}

#[test]
fn cast_bool_from_string() -> Result<()> {
    let mut record = Record::new();
    record.insert("active", "true".into());

    let value = Evaluator::evaluate(&field("active").cast_bool(), &record)?;

    assert_eq!(value, Value::Boolean(true));

    Ok(())
}

#[test]
fn cast_string_from_int() -> Result<()> {
    let mut record = Record::new();
    record.insert("id", 100.into());

    let value = Evaluator::evaluate(&field("id").cast_string(), &record)?;

    assert_eq!(value, Value::String("100".to_string()));

    Ok(())
}

#[test]
fn cast_null_remains_null() -> Result<()> {
    let mut record = Record::new();
    record.insert("value", Value::Null);

    let value = Evaluator::evaluate(&field("value").cast_string(), &record)?;

    assert_eq!(value, Value::Null);

    Ok(())
}

#[test]
fn cast_int_invalid_string() {
    let mut record = Record::new();
    record.insert("age", "abc".into());

    let result = Evaluator::evaluate(&field("age").cast_int(), &record);

    assert!(result.is_err());
}

#[test]
fn cast_bool_invalid_string() {
    let mut record = Record::new();
    record.insert("active", "yes".into());

    let result = Evaluator::evaluate(&field("active").cast_bool(), &record);

    assert!(result.is_err());
}
