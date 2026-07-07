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
                let left = Self::evaluate(left, record)?;
                let right = Self::evaluate(right, record)?;

                Self::evaluate_binary(left, *op, right)
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

            And => match (left, right) {
                (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a && b)),
                _ => Err(anyhow!("AND requires booleans")),
            },

            Or => match (left, right) {
                (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a || b)),
                _ => Err(anyhow!("OR requires booleans")),
            },
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

        (Value::String(a), Value::String(b)) => a.cmp(&b),

        (Value::Boolean(a), Value::Boolean(b)) => a.cmp(&b),

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
