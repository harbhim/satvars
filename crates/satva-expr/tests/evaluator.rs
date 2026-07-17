use satva_expr::{Evaluator, field, lit};
use satva_types::{Record, Value};

fn record() -> Record {
    let mut record = Record::new();

    record.insert("age", 25.into());
    record.insert("salary", 75_000.into());
    record.insert("bonus", 2_500.0.into());
    record.insert("active", true.into());
    record.insert("name", "John".into());
    record.insert("department", "Engineering".into());

    record
}

fn record_with_nulls() -> Record {
    let mut record = Record::new();

    record.insert("age", Value::Null);
    record.insert("active", Value::Null);
    record.insert("name", Value::Null);

    record
}

#[test]
fn evaluate_integer_literal() {
    let result = Evaluator::evaluate(&lit(42), &record()).unwrap();

    assert_eq!(result, Value::Int64(42));
}

#[test]
fn evaluate_boolean_literal() {
    let result = Evaluator::evaluate(&lit(true), &record()).unwrap();

    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn evaluate_string_literal() {
    let result = Evaluator::evaluate(&lit("hello"), &record()).unwrap();

    assert_eq!(result, Value::String("hello".to_string()));
}

#[test]
fn evaluate_field_lookup() {
    let result = Evaluator::evaluate(&field("salary"), &record()).unwrap();

    assert_eq!(result, Value::Int64(75_000));
}

#[test]
fn evaluate_addition() {
    let expr = lit(10).plus(lit(5));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Int64(15));
}

#[test]
fn evaluate_subtraction() {
    let expr = lit(10).minus(lit(3));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Int64(7));
}

#[test]
fn evaluate_multiplication() {
    let expr = lit(6).times(lit(7));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Int64(42));
}

#[test]
fn evaluate_division() {
    let expr = lit(20).divide_by(lit(4));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Int64(5));
}

#[test]
fn evaluate_modulo() {
    let expr = lit(10).modulo(lit(3));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Int64(1));
}

#[test]
fn evaluate_mixed_numeric_addition() {
    let expr = lit(10).plus(lit(2.5));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Float64(12.5));
}

#[test]
fn evaluate_string_concatenation() {
    let expr = lit("Hello ").plus(lit("World"));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::String("Hello World".to_string()));
}

#[test]
fn evaluate_greater_than() {
    let expr = field("age").greater_than(lit(18));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn evaluate_less_than() {
    let expr = field("age").less_than(lit(30));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn evaluate_equal() {
    let expr = field("department").equal_to(lit("Engineering"));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn evaluate_not_equal() {
    let expr = field("department").not_equal_to(lit("HR"));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn evaluate_and() {
    let expr = field("active").and(field("age").greater_than(lit(18)));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn evaluate_or() {
    let expr = field("active").or(lit(false));

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Boolean(true));
}

#[test]
fn evaluate_logical_not() {
    let expr = field("active").logical_not();

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Boolean(false));
}

#[test]
fn evaluate_negate_integer() {
    let expr = lit(10).negate();

    let result = Evaluator::evaluate(&expr, &record()).unwrap();

    assert_eq!(result, Value::Int64(-10));
}

#[test]
fn missing_field_returns_error() {
    let result = Evaluator::evaluate(&field("missing"), &record());

    assert!(result.is_err());
}

#[test]
fn invalid_addition_returns_error() {
    let expr = lit(true).plus(lit(10));

    let result = Evaluator::evaluate(&expr, &record());

    assert!(result.is_err());
}

#[test]
fn invalid_comparison_returns_error() {
    let expr = lit("abc").greater_than(lit(10));

    let result = Evaluator::evaluate(&expr, &record());

    assert!(result.is_err());
}

// --- Short-circuit tests ---

#[test]
fn and_short_circuits_on_false_left() {
    // false && (side_effect) — the right side should never be evaluated
    let expr = lit(false).and(field("nonexistent"));
    let result = Evaluator::evaluate(&expr, &record());
    assert_eq!(result.unwrap(), Value::Boolean(false));
}

#[test]
fn and_does_not_short_circuit_on_true_left() {
    // true && nonexistent — right side is evaluated and should error
    let expr = lit(true).and(field("nonexistent"));
    let result = Evaluator::evaluate(&expr, &record());
    assert!(result.is_err());
}

#[test]
fn or_short_circuits_on_true_left() {
    // true || (side_effect) — the right side should never be evaluated
    let expr = lit(true).or(field("nonexistent"));
    let result = Evaluator::evaluate(&expr, &record());
    assert_eq!(result.unwrap(), Value::Boolean(true));
}

#[test]
fn or_does_not_short_circuit_on_false_left() {
    // false || nonexistent — right side is evaluated and should error
    let expr = lit(false).or(field("nonexistent"));
    let result = Evaluator::evaluate(&expr, &record());
    assert!(result.is_err());
}

#[test]
fn and_short_circuit_protects_null_check() {
    // is_not_null(age) && age > 5 — if age is null, short-circuit prevents comparison
    let expr = field("age")
        .is_not_null()
        .and(field("age").greater_than(lit(5)));
    let result = Evaluator::evaluate(&expr, &record_with_nulls());
    assert_eq!(result.unwrap(), Value::Boolean(false));
}

#[test]
fn or_short_circuit_protects_null_check() {
    // active == true || nonexistent_field > 0 — left is true, short-circuit
    let expr = field("active")
        .equal_to(lit(true))
        .or(field("nonexistent").greater_than(lit(0)));
    let result = Evaluator::evaluate(&expr, &record_with_nulls());
    // active is null, so active == true is null == true = false (not true)
    // So it doesn't short-circuit on the OR
    assert!(result.is_err());
}

// --- Null comparison tests ---

#[test]
fn null_compared_with_int_returns_false() {
    let expr = field("age").greater_than(lit(5));
    let result = Evaluator::evaluate(&expr, &record_with_nulls());
    assert_eq!(result.unwrap(), Value::Boolean(false));
}

#[test]
fn null_compared_with_null_returns_false() {
    let expr = field("age").greater_than(field("active"));
    let result = Evaluator::evaluate(&expr, &record_with_nulls());
    assert_eq!(result.unwrap(), Value::Boolean(false));
}

#[test]
fn null_equal_to_null_returns_true() {
    let expr = field("age").equal_to(field("active"));
    let result = Evaluator::evaluate(&expr, &record_with_nulls());
    assert_eq!(result.unwrap(), Value::Boolean(true));
}

#[test]
fn null_equal_to_value_returns_false() {
    let expr = field("age").equal_to(lit(5));
    let result = Evaluator::evaluate(&expr, &record_with_nulls());
    assert_eq!(result.unwrap(), Value::Boolean(false));
}

#[test]
fn null_not_equal_to_value_returns_true() {
    let expr = field("age").not_equal_to(lit(5));
    let result = Evaluator::evaluate(&expr, &record_with_nulls());
    assert_eq!(result.unwrap(), Value::Boolean(true));
}

#[test]
fn and_with_null_active_skips_gracefully() {
    // active is null, so active == true is false (null comparison returns false)
    let expr = field("active")
        .equal_to(lit(true))
        .and(field("age").greater_than(lit(0)));
    let result = Evaluator::evaluate(&expr, &record_with_nulls());
    assert_eq!(result.unwrap(), Value::Boolean(false));
}
