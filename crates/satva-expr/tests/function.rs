use anyhow::Result;
use satva_expr::{Evaluator, concat, field, lit};
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
