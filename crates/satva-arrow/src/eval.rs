use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use satva_expr::{BinaryOperator, Evaluator, Expression, Function, UnaryOperator};

use crate::array::ColumnArray;
use crate::batch::RecordBatch;
use crate::kernels::{
    BoolSide, EvalValue, Fast, apply_negate, apply_not, bool_column, bool_side, fast_column_scalar,
    fast_columns, fast_scalar_column, map_utf8, uniform_bool, zip_bool,
};

/// Keeps rows whose expression is boolean true. Other booleans are skipped.
pub fn filter_batch(batch: &RecordBatch, expr: &Expression) -> Result<(RecordBatch, usize)> {
    if batch.num_rows() == 0 {
        return Ok((batch.clone(), 0));
    }
    let mask = evaluate(expr, batch)?;
    match mask {
        EvalValue::Scalar(satva_types::Value::Boolean(true)) => Ok((batch.clone(), 0)),
        EvalValue::Scalar(satva_types::Value::Boolean(false)) => {
            Ok((batch.empty_like(), batch.num_rows()))
        }
        EvalValue::Scalar(value) => Err(anyhow!(
            "Filter expression returned '{value}', expected Boolean"
        )),
        EvalValue::Column(column) => {
            let ColumnArray::Boolean { values, nulls } = column.as_ref() else {
                return Err(anyhow!(
                    "Filter expression returned '{}', expected Boolean",
                    column_type_name(column.as_ref())
                ));
            };
            if nulls
                .as_ref()
                .is_some_and(|flags| flags.iter().any(|is_null| *is_null))
            {
                return Err(anyhow!(
                    "Filter expression returned 'null', expected Boolean"
                ));
            }
            let indices: Vec<usize> = values
                .iter()
                .enumerate()
                .filter_map(|(index, keep)| keep.then_some(index))
                .collect();
            let skipped = batch.num_rows() - indices.len();
            Ok((batch.take(&indices), skipped))
        }
    }
}

/// Keeps columns named in `fields`, in their existing order. Missing names are ignored.
pub fn select_columns(batch: &RecordBatch, fields: &[&str]) -> Result<RecordBatch> {
    let selected: HashSet<&str> = fields.iter().copied().collect();
    let columns = batch
        .columns()
        .iter()
        .filter(|column| selected.contains(column.name()))
        .cloned()
        .collect();
    RecordBatch::try_with_rows(columns, batch.num_rows())
}

/// Appends or replaces `field` with the expression result.
pub fn set_column(batch: &RecordBatch, field: &str, expr: &Expression) -> Result<RecordBatch> {
    let value = evaluate(expr, batch)?;
    let data = match value {
        EvalValue::Scalar(scalar) => Arc::new(ColumnArray::broadcast(&scalar, batch.num_rows())),
        EvalValue::Column(column) => {
            if column.len() != batch.num_rows() {
                return Err(anyhow!(
                    "Expression length {} does not match batch length {}",
                    column.len(),
                    batch.num_rows()
                ));
            }
            column
        }
    };
    let column = crate::batch::Column::from_shared(field, data);
    let mut columns: Vec<_> = batch.columns().to_vec();
    if let Some(existing) = columns.iter_mut().find(|item| item.name() == field) {
        *existing = column;
    } else {
        columns.push(column);
    }
    RecordBatch::try_with_rows(columns, batch.num_rows())
}

fn evaluate(expr: &Expression, batch: &RecordBatch) -> Result<EvalValue> {
    match expr {
        Expression::Literal(value) => Ok(EvalValue::Scalar(value.clone())),
        Expression::Field(name) => {
            let column = batch
                .column(name)
                .ok_or_else(|| anyhow!("Field '{name}' not found"))?;
            Ok(EvalValue::Column(Arc::clone(column.data())))
        }
        Expression::Unary { op, expr } => eval_unary(*op, evaluate(expr, batch)?),
        Expression::Binary { left, op, right } => eval_binary(*op, left, right, batch),
        Expression::Function {
            function,
            arguments,
        } => eval_function(*function, arguments, batch),
    }
}

fn eval_unary(op: UnaryOperator, value: EvalValue) -> Result<EvalValue> {
    let fast = match op {
        UnaryOperator::Not => apply_not(&value)?,
        UnaryOperator::Negate => apply_negate(&value)?,
    };
    if let Fast::Done(done) = fast {
        return Ok(done);
    }
    match value {
        EvalValue::Scalar(scalar) => Ok(EvalValue::Scalar(Evaluator::evaluate_unary(op, scalar)?)),
        EvalValue::Column(column) => {
            let mut values = Vec::with_capacity(column.len());
            for row in 0..column.len() {
                values.push(Evaluator::evaluate_unary(op, column.value_at(row))?);
            }
            Ok(EvalValue::Column(Arc::new(ColumnArray::from_values(
                &values,
            )?)))
        }
    }
}

fn eval_binary(
    op: BinaryOperator,
    left: &Expression,
    right: &Expression,
    batch: &RecordBatch,
) -> Result<EvalValue> {
    match op {
        BinaryOperator::And => eval_and(left, right, batch),
        BinaryOperator::Or => eval_or(left, right, batch),
        _ => {
            let left = evaluate(left, batch)?;
            let right = evaluate(right, batch)?;
            eval_binary_values(op, &left, &right)
        }
    }
}

fn eval_and(left: &Expression, right: &Expression, batch: &RecordBatch) -> Result<EvalValue> {
    let left = evaluate(left, batch)?;
    match uniform_bool(&left, "AND")? {
        Some(false) => Ok(EvalValue::Scalar(satva_types::Value::Boolean(false))),
        Some(true) => ensure_bool(evaluate(right, batch)?, "AND"),
        None => combine_bool(&left, &evaluate(right, batch)?, true, "AND"),
    }
}

fn eval_or(left: &Expression, right: &Expression, batch: &RecordBatch) -> Result<EvalValue> {
    let left = evaluate(left, batch)?;
    match uniform_bool(&left, "OR")? {
        Some(true) => Ok(EvalValue::Scalar(satva_types::Value::Boolean(true))),
        Some(false) => ensure_bool(evaluate(right, batch)?, "OR"),
        None => combine_bool(&left, &evaluate(right, batch)?, false, "OR"),
    }
}

fn ensure_bool(value: EvalValue, operator: &str) -> Result<EvalValue> {
    match bool_side(&value, operator)? {
        BoolSide::Scalar(flag) => Ok(EvalValue::Scalar(satva_types::Value::Boolean(flag))),
        BoolSide::Column(_) => Ok(value),
    }
}

fn combine_bool(
    left: &EvalValue,
    right: &EvalValue,
    and: bool,
    operator: &str,
) -> Result<EvalValue> {
    match (bool_side(left, operator)?, bool_side(right, operator)?) {
        (BoolSide::Scalar(lhs), BoolSide::Scalar(rhs)) => {
            Ok(EvalValue::Scalar(satva_types::Value::Boolean(if and {
                lhs && rhs
            } else {
                lhs || rhs
            })))
        }
        (BoolSide::Scalar(false), _) if and => {
            Ok(EvalValue::Scalar(satva_types::Value::Boolean(false)))
        }
        (BoolSide::Scalar(true), _) if !and => {
            Ok(EvalValue::Scalar(satva_types::Value::Boolean(true)))
        }
        (BoolSide::Scalar(true), BoolSide::Column(_)) if and => Ok(left_or_right_column(right)),
        (BoolSide::Scalar(false), BoolSide::Column(_)) if !and => Ok(left_or_right_column(right)),
        (BoolSide::Column(_), BoolSide::Scalar(true)) if and => Ok(left_or_right_column(left)),
        (BoolSide::Column(_), BoolSide::Scalar(false)) if !and => Ok(left_or_right_column(left)),
        (BoolSide::Column(_), BoolSide::Scalar(false)) if and => {
            Ok(EvalValue::Scalar(satva_types::Value::Boolean(false)))
        }
        (BoolSide::Column(_), BoolSide::Scalar(true)) if !and => {
            Ok(EvalValue::Scalar(satva_types::Value::Boolean(true)))
        }
        (BoolSide::Column(lhs), BoolSide::Column(rhs)) => Ok(bool_column(zip_bool(lhs, rhs, and)?)),
        (BoolSide::Scalar(_), BoolSide::Column(_)) | (BoolSide::Column(_), BoolSide::Scalar(_)) => {
            Err(anyhow!("{operator} requires booleans"))
        }
    }
}

fn left_or_right_column(value: &EvalValue) -> EvalValue {
    value.clone()
}

fn eval_binary_values(
    op: BinaryOperator,
    left: &EvalValue,
    right: &EvalValue,
) -> Result<EvalValue> {
    let fast = match (left, right) {
        (EvalValue::Column(lhs), EvalValue::Scalar(rhs)) => fast_column_scalar(op, lhs, rhs)?,
        (EvalValue::Scalar(lhs), EvalValue::Column(rhs)) => fast_scalar_column(op, lhs, rhs)?,
        (EvalValue::Column(lhs), EvalValue::Column(rhs)) => fast_columns(op, lhs, rhs)?,
        (EvalValue::Scalar(_), EvalValue::Scalar(_)) => Fast::Fallback,
    };
    if let Fast::Done(done) = fast {
        return Ok(done);
    }
    if let (EvalValue::Scalar(lhs), EvalValue::Scalar(rhs)) = (left, right) {
        return Ok(EvalValue::Scalar(Evaluator::evaluate_binary(
            lhs.clone(),
            op,
            rhs.clone(),
        )?));
    }
    let len = aligned_len(left, right)?;
    let mut values = Vec::with_capacity(len);
    for row in 0..len {
        values.push(Evaluator::evaluate_binary(
            left.value_at(row),
            op,
            right.value_at(row),
        )?);
    }
    Ok(EvalValue::Column(Arc::new(ColumnArray::from_values(
        &values,
    )?)))
}

fn aligned_len(left: &EvalValue, right: &EvalValue) -> Result<usize> {
    match (left.column_len(), right.column_len()) {
        (Some(len), Some(other)) if len == other => Ok(len),
        (Some(len), None) | (None, Some(len)) => Ok(len),
        (Some(len), Some(other)) => Err(anyhow!("Column lengths {len} and {other} do not match")),
        (None, None) => Err(anyhow!("Missing column length")),
    }
}

fn eval_function(
    function: Function,
    arguments: &[Expression],
    batch: &RecordBatch,
) -> Result<EvalValue> {
    if arguments.len() == 1
        && matches!(function, Function::Upper | Function::Lower | Function::Trim)
    {
        let value = evaluate(&arguments[0], batch)?;
        if let EvalValue::Column(column) = &value {
            let message = match function {
                Function::Upper => "upper() expects one string argument",
                Function::Lower => "lower() expects one string argument",
                Function::Trim => "trim() expects one string argument",
                _ => "function expects one string argument",
            };
            let mapped = map_utf8(column, message, |text| match function {
                Function::Upper => text.to_uppercase(),
                Function::Lower => text.to_lowercase(),
                Function::Trim => text.trim().to_string(),
                _ => text.to_string(),
            })?;
            if let Fast::Done(done) = mapped {
                return Ok(done);
            }
        }
        return zip_function(function, &[value], batch.num_rows());
    }
    let mut evaluated = Vec::with_capacity(arguments.len());
    for argument in arguments {
        evaluated.push(evaluate(argument, batch)?);
    }
    zip_function(function, &evaluated, batch.num_rows())
}

fn zip_function(
    function: Function,
    arguments: &[EvalValue],
    batch_len: usize,
) -> Result<EvalValue> {
    if arguments.iter().all(|value| value.column_len().is_none()) {
        let values = arguments
            .iter()
            .map(|value| value.value_at(0))
            .collect::<Vec<_>>();
        return Ok(EvalValue::Scalar(Evaluator::evaluate_function(
            function, values,
        )?));
    }
    let len = arguments
        .iter()
        .find_map(EvalValue::column_len)
        .unwrap_or(batch_len);
    let mut rows = Vec::with_capacity(len);
    for row in 0..len {
        let values = arguments.iter().map(|value| value.value_at(row)).collect();
        rows.push(Evaluator::evaluate_function(function, values)?);
    }
    Ok(EvalValue::Column(Arc::new(ColumnArray::from_values(
        &rows,
    )?)))
}

fn column_type_name(column: &ColumnArray) -> &'static str {
    match column {
        ColumnArray::Null { .. } => "null",
        ColumnArray::Int64 { .. } => "int64",
        ColumnArray::Float64 { .. } => "float64",
        ColumnArray::Boolean { .. } => "boolean",
        ColumnArray::Utf8 { .. } => "string",
    }
}
