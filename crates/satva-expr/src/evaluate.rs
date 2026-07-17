use anyhow::{Result, anyhow};

use satva_types::{Record, Value};

use crate::{BinaryOperator, Expression, Function, UnaryOperator};

pub struct Evaluator;

impl Evaluator {
    pub fn evaluate(expr: &Expression, record: &Record) -> Result<Value> {
        match expr {
            Expression::Literal(value) => Ok(value.clone()),

            Expression::Field(name) => record
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow!("Field '{}' not found", name)),

            Expression::Unary { op, expr } => {
                let value = Self::evaluate(expr, record)?;

                Self::evaluate_unary(*op, value)
            }

            Expression::Binary { left, op, right } => {
                use BinaryOperator::*;

                match op {
                    And => Self::evaluate_and(left, right, record),
                    Or => Self::evaluate_or(left, right, record),
                    _ => {
                        let left = Self::evaluate(left, record)?;
                        let right = Self::evaluate(right, record)?;
                        Self::evaluate_binary(left, *op, right)
                    }
                }
            }

            Expression::Function {
                function,
                arguments,
            } => {
                let arguments = arguments
                    .iter()
                    .map(|expr| Self::evaluate(expr, record))
                    .collect::<Result<Vec<_>>>()?;

                Self::evaluate_function(*function, arguments)
            }
        }
    }

    fn evaluate_function(function: Function, arguments: Vec<Value>) -> Result<Value> {
        match function {
            Function::Upper => upper(arguments),
            Function::Lower => lower(arguments),
            Function::Trim => trim(arguments),
            Function::Length => length(arguments),
            Function::Concat => concat(arguments),
            Function::Coalesce => coalesce(arguments),
            Function::IsNull => is_null(arguments),
            Function::IsNotNull => is_not_null(arguments),
            Function::CastInt => cast_int(arguments),
            Function::CastFloat => cast_float(arguments),
            Function::CastBool => cast_bool(arguments),
            Function::CastString => cast_string(arguments),
        }
    }

    fn evaluate_and(left: &Expression, right: &Expression, record: &Record) -> Result<Value> {
        let left = Self::evaluate(left, record)?;
        match left {
            Value::Boolean(false) => Ok(Value::Boolean(false)),
            Value::Boolean(true) => {
                let right = Self::evaluate(right, record)?;
                match right {
                    Value::Boolean(v) => Ok(Value::Boolean(v)),
                    _ => Err(anyhow!("AND requires booleans")),
                }
            }
            _ => Err(anyhow!("AND requires booleans")),
        }
    }

    fn evaluate_or(left: &Expression, right: &Expression, record: &Record) -> Result<Value> {
        let left = Self::evaluate(left, record)?;
        match left {
            Value::Boolean(true) => Ok(Value::Boolean(true)),
            Value::Boolean(false) => {
                let right = Self::evaluate(right, record)?;
                match right {
                    Value::Boolean(v) => Ok(Value::Boolean(v)),
                    _ => Err(anyhow!("OR requires booleans")),
                }
            }
            _ => Err(anyhow!("OR requires booleans")),
        }
    }

    fn evaluate_unary(op: UnaryOperator, value: Value) -> Result<Value> {
        match (op, value) {
            (UnaryOperator::Not, Value::Boolean(v)) => Ok(Value::Boolean(!v)),

            (UnaryOperator::Negate, Value::Int64(v)) => Ok(Value::Int64(-v)),

            (UnaryOperator::Negate, Value::Float64(v)) => Ok(Value::Float64(-v)),

            _ => Err(anyhow!("Invalid unary operation")),
        }
    }

    fn evaluate_binary(left: Value, op: BinaryOperator, right: Value) -> Result<Value> {
        use BinaryOperator::*;

        match op {
            Add => arithmetic(left, right, |a, b| a + b, |a, b| a + b),

            Subtract => arithmetic(left, right, |a, b| a - b, |a, b| a - b),

            Multiply => arithmetic(left, right, |a, b| a * b, |a, b| a * b),

            Divide => arithmetic(left, right, |a, b| a / b, |a, b| a / b),

            Modulo => match (left, right) {
                (Value::Int64(a), Value::Int64(b)) => Ok(Value::Int64(a % b)),
                _ => Err(anyhow!("Modulo requires integers")),
            },

            Equal => Ok(Value::Boolean(left == right)),

            NotEqual => Ok(Value::Boolean(left != right)),

            GreaterThan => compare(left, right, |o| o.is_gt()),

            GreaterThanOrEqual => compare(left, right, |o| o.is_ge()),

            LessThan => compare(left, right, |o| o.is_lt()),

            LessThanOrEqual => compare(left, right, |o| o.is_le()),

            And | Or => unreachable!("And/Or are handled in evaluate()"),
        }
    }
}

fn arithmetic(
    left: Value,
    right: Value,
    int_op: impl Fn(i64, i64) -> i64,
    float_op: impl Fn(f64, f64) -> f64,
) -> Result<Value> {
    match (left, right) {
        (Value::Int64(a), Value::Int64(b)) => Ok(Value::Int64(int_op(a, b))),

        (Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(float_op(a, b))),

        (Value::Int64(a), Value::Float64(b)) => Ok(Value::Float64(float_op(a as f64, b))),

        (Value::Float64(a), Value::Int64(b)) => Ok(Value::Float64(float_op(a, b as f64))),

        (Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),

        _ => Err(anyhow!("Invalid arithmetic operands")),
    }
}

fn compare(
    left: Value,
    right: Value,
    predicate: impl Fn(std::cmp::Ordering) -> bool,
) -> Result<Value> {
    let ordering = match (left, right) {
        (Value::Int64(a), Value::Int64(b)) => a.cmp(&b),

        (Value::Float64(a), Value::Float64(b)) => a
            .partial_cmp(&b)
            .ok_or_else(|| anyhow!("Cannot compare NaN"))?,

        (Value::Int64(a), Value::Float64(b)) => (a as f64)
            .partial_cmp(&b)
            .ok_or_else(|| anyhow!("Cannot compare NaN"))?,

        (Value::Float64(a), Value::Int64(b)) => a
            .partial_cmp(&(b as f64))
            .ok_or_else(|| anyhow!("Cannot compare NaN"))?,

        (Value::String(a), Value::String(b)) => a.cmp(&b),

        (Value::Boolean(a), Value::Boolean(b)) => a.cmp(&b),

        (Value::Null, _) | (_, Value::Null) => return Ok(Value::Boolean(false)),

        _ => {
            return Err(anyhow!("Cannot compare different value types"));
        }
    };

    Ok(Value::Boolean(predicate(ordering)))
}

fn upper(arguments: Vec<Value>) -> Result<Value> {
    match arguments.as_slice() {
        [Value::String(value)] => Ok(Value::String(value.to_uppercase())),
        _ => Err(anyhow!("upper() expects one string argument")),
    }
}

fn lower(arguments: Vec<Value>) -> Result<Value> {
    match arguments.as_slice() {
        [Value::String(value)] => Ok(Value::String(value.to_lowercase())),
        _ => Err(anyhow!("lower() expects one string argument")),
    }
}

fn trim(arguments: Vec<Value>) -> Result<Value> {
    match arguments.as_slice() {
        [Value::String(value)] => Ok(Value::String(value.trim().to_string())),
        _ => Err(anyhow!("trim() expects one string argument")),
    }
}

fn length(arguments: Vec<Value>) -> Result<Value> {
    match arguments.as_slice() {
        [Value::String(value)] => Ok(Value::Int64(value.chars().count() as i64)),
        _ => Err(anyhow!("length() expects one string argument")),
    }
}

fn concat(arguments: Vec<Value>) -> Result<Value> {
    let mut result = String::new();

    for value in arguments {
        match value {
            Value::String(text) => result.push_str(&text),
            _ => return Err(anyhow!("concat() expects string arguments")),
        }
    }

    Ok(Value::String(result))
}

fn coalesce(arguments: Vec<Value>) -> Result<Value> {
    if arguments.is_empty() {
        return Err(anyhow!("coalesce() expects at least one argument"));
    }

    for value in arguments {
        if value != Value::Null {
            return Ok(value);
        }
    }

    Ok(Value::Null)
}

fn is_null(arguments: Vec<Value>) -> Result<Value> {
    match arguments.as_slice() {
        [value] => Ok(Value::Boolean(*value == Value::Null)),
        _ => Err(anyhow!("is_null() expects one argument")),
    }
}

fn is_not_null(arguments: Vec<Value>) -> Result<Value> {
    match arguments.as_slice() {
        [value] => Ok(Value::Boolean(*value != Value::Null)),
        _ => Err(anyhow!("is_not_null() expects one argument")),
    }
}

fn cast_int(arguments: Vec<Value>) -> Result<Value> {
    match arguments.as_slice() {
        [Value::Int64(v)] => Ok(Value::Int64(*v)),

        [Value::Float64(v)] => Ok(Value::Int64(*v as i64)),

        [Value::Boolean(v)] => Ok(Value::Int64(i64::from(*v))),

        [Value::String(v)] => v.parse::<i64>().map(Value::Int64).map_err(Into::into),

        [Value::Null] => Ok(Value::Null),

        _ => Err(anyhow!("cast_int() expects one argument")),
    }
}

fn cast_float(arguments: Vec<Value>) -> Result<Value> {
    match arguments.as_slice() {
        [Value::Float64(v)] => Ok(Value::Float64(*v)),

        [Value::Int64(v)] => Ok(Value::Float64(*v as f64)),

        [Value::Boolean(v)] => Ok(Value::Float64(if *v { 1.0 } else { 0.0 })),

        [Value::String(v)] => v.parse::<f64>().map(Value::Float64).map_err(Into::into),

        [Value::Null] => Ok(Value::Null),

        _ => Err(anyhow!("cast_float() expects one argument")),
    }
}

fn cast_bool(arguments: Vec<Value>) -> Result<Value> {
    match arguments.as_slice() {
        [Value::Boolean(v)] => Ok(Value::Boolean(*v)),

        [Value::Int64(v)] => Ok(Value::Boolean(*v != 0)),

        [Value::Float64(v)] => Ok(Value::Boolean(*v != 0.0)),

        [Value::String(v)] => v.parse::<bool>().map(Value::Boolean).map_err(Into::into),

        [Value::Null] => Ok(Value::Null),

        _ => Err(anyhow!("cast_bool() expects one argument")),
    }
}

fn cast_string(arguments: Vec<Value>) -> Result<Value> {
    match arguments.as_slice() {
        [Value::String(v)] => Ok(Value::String(v.clone())),

        [Value::Null] => Ok(Value::Null),

        [value] => Ok(Value::String(value.to_string())),

        _ => Err(anyhow!("cast_string() expects one argument")),
    }
}
