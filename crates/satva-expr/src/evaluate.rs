use anyhow::{Result, anyhow};
use satva_types::{Record, Value};

use crate::expr::{BinaryOperator, Expression, UnaryOperator};

/// Evaluates an expression against a record.
pub struct Evaluator;

impl Evaluator {
    pub fn evaluate(expr: &Expression, record: &Record) -> Result<Value> {
        match expr {
            Expression::Literal(value) => Ok(value.clone()),

            Expression::Field(name) => record
                .fields
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow!("Field '{}' not found", name)),

            Expression::Unary { op, expr } => {
                let value = Self::evaluate(expr, record)?;

                match (op, value) {
                    (UnaryOperator::Not, Value::Boolean(v)) => Ok(Value::Boolean(!v)),

                    (UnaryOperator::Negate, Value::Int64(v)) => Ok(Value::Int64(-v)),

                    (UnaryOperator::Negate, Value::Float64(v)) => Ok(Value::Float64(-v)),

                    _ => Err(anyhow!("Invalid unary expression")),
                }
            }

            Expression::Binary { left, op, right } => {
                let left = Self::evaluate(left, record)?;
                let right = Self::evaluate(right, record)?;

                Self::evaluate_binary(left, *op, right)
            }
        }
    }

    fn evaluate_binary(left: Value, op: BinaryOperator, right: Value) -> Result<Value> {
        match op {
            BinaryOperator::Add => Self::add(left, right),
            BinaryOperator::Subtract => Self::subtract(left, right),
            BinaryOperator::Multiply => Self::multiply(left, right),
            BinaryOperator::Divide => Self::divide(left, right),
            BinaryOperator::Modulo => Self::modulo(left, right),

            BinaryOperator::Equal => Ok(Value::Boolean(left == right)),
            BinaryOperator::NotEqual => Ok(Value::Boolean(left != right)),

            BinaryOperator::GreaterThan => Ok(Value::Boolean(compare(left, right)? > 0)),

            BinaryOperator::GreaterThanOrEqual => Ok(Value::Boolean(compare(left, right)? >= 0)),

            BinaryOperator::LessThan => Ok(Value::Boolean(compare(left, right)? < 0)),

            BinaryOperator::LessThanOrEqual => Ok(Value::Boolean(compare(left, right)? <= 0)),

            BinaryOperator::And => match (left, right) {
                (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a && b)),
                _ => Err(anyhow!("AND requires booleans")),
            },

            BinaryOperator::Or => match (left, right) {
                (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a || b)),
                _ => Err(anyhow!("OR requires booleans")),
            },
        }
    }

    fn add(left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Int64(a), Value::Int64(b)) => Ok(Value::Int64(a + b)),
            (Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(a + b)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
            _ => Err(anyhow!("Invalid addition")),
        }
    }

    fn subtract(left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Int64(a), Value::Int64(b)) => Ok(Value::Int64(a - b)),
            (Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(a - b)),
            _ => Err(anyhow!("Invalid subtraction")),
        }
    }

    fn multiply(left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Int64(a), Value::Int64(b)) => Ok(Value::Int64(a * b)),
            (Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(a * b)),
            _ => Err(anyhow!("Invalid multiplication")),
        }
    }

    fn divide(left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Int64(a), Value::Int64(b)) => Ok(Value::Int64(a / b)),
            (Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(a / b)),
            _ => Err(anyhow!("Invalid division")),
        }
    }

    fn modulo(left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Int64(a), Value::Int64(b)) => Ok(Value::Int64(a % b)),
            _ => Err(anyhow!("Invalid modulo")),
        }
    }
}

fn compare(left: Value, right: Value) -> Result<i8> {
    match (left, right) {
        (Value::Int64(a), Value::Int64(b)) => Ok(a.cmp(&b) as i8),
        (Value::Float64(a), Value::Float64(b)) => Ok(a.partial_cmp(&b).unwrap() as i8),
        (Value::String(a), Value::String(b)) => Ok(a.cmp(&b) as i8),
        _ => Err(anyhow!("Cannot compare values")),
    }
}
