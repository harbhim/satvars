#![allow(clippy::ref_option)]

use std::cmp::Ordering;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use satva_expr::BinaryOperator;
use satva_types::Value;

use crate::array::{ColumnArray, is_null_at};

#[derive(Debug, Clone)]
pub(crate) enum EvalValue {
    Scalar(Value),
    Column(Arc<ColumnArray>),
}

pub(crate) enum Fast {
    Done(EvalValue),
    Fallback,
}

impl EvalValue {
    pub(crate) fn value_at(&self, index: usize) -> Value {
        match self {
            Self::Scalar(value) => value.clone(),
            Self::Column(column) => column.value_at(index),
        }
    }

    pub(crate) fn column_len(&self) -> Option<usize> {
        match self {
            Self::Scalar(_) => None,
            Self::Column(column) => Some(column.len()),
        }
    }
}

pub(crate) fn bool_column(values: Vec<bool>) -> EvalValue {
    EvalValue::Column(Arc::new(ColumnArray::Boolean {
        values,
        nulls: None,
    }))
}

pub(crate) fn column_value(data: ColumnArray) -> EvalValue {
    EvalValue::Column(Arc::new(data))
}

pub(crate) enum BoolSide<'a> {
    Scalar(bool),
    Column(&'a [bool]),
}

pub(crate) fn bool_side<'a>(value: &'a EvalValue, operator: &str) -> Result<BoolSide<'a>> {
    match value {
        EvalValue::Scalar(Value::Boolean(flag)) => Ok(BoolSide::Scalar(*flag)),
        EvalValue::Column(column) => match column.as_ref() {
            ColumnArray::Boolean { values, nulls } => {
                if nulls
                    .as_ref()
                    .is_some_and(|flags| flags.iter().any(|is_null| *is_null))
                {
                    Err(anyhow!("{operator} requires booleans"))
                } else {
                    Ok(BoolSide::Column(values))
                }
            }
            _ => Err(anyhow!("{operator} requires booleans")),
        },
        EvalValue::Scalar(_) => Err(anyhow!("{operator} requires booleans")),
    }
}

pub(crate) fn uniform_bool(value: &EvalValue, operator: &str) -> Result<Option<bool>> {
    match bool_side(value, operator)? {
        BoolSide::Scalar(flag) => Ok(Some(flag)),
        BoolSide::Column(values) => {
            let mut iter = values.iter().copied();
            let Some(first) = iter.next() else {
                return Ok(Some(true));
            };
            if iter.all(|flag| flag == first) {
                Ok(Some(first))
            } else {
                Ok(None)
            }
        }
    }
}

pub(crate) fn zip_bool(left: &[bool], right: &[bool], and: bool) -> Result<Vec<bool>> {
    if left.len() != right.len() {
        return Err(anyhow!(
            "Column lengths {} and {} do not match",
            left.len(),
            right.len()
        ));
    }
    Ok(left
        .iter()
        .zip(right)
        .map(|(lhs, rhs)| if and { *lhs && *rhs } else { *lhs || *rhs })
        .collect())
}

pub(crate) fn fast_column_scalar(
    op: BinaryOperator,
    column: &ColumnArray,
    scalar: &Value,
) -> Result<Fast> {
    match op {
        BinaryOperator::Equal => Ok(Fast::Done(eq_column_scalar(column, scalar, false))),
        BinaryOperator::NotEqual => Ok(Fast::Done(eq_column_scalar(column, scalar, true))),
        BinaryOperator::GreaterThan
        | BinaryOperator::GreaterThanOrEqual
        | BinaryOperator::LessThan
        | BinaryOperator::LessThanOrEqual => ord_column_scalar(op, column, scalar),
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Multiply
        | BinaryOperator::Divide
        | BinaryOperator::Modulo => arith_column_scalar(op, column, scalar),
        BinaryOperator::And | BinaryOperator::Or => Ok(Fast::Fallback),
    }
}

pub(crate) fn fast_scalar_column(
    op: BinaryOperator,
    scalar: &Value,
    column: &ColumnArray,
) -> Result<Fast> {
    match op {
        BinaryOperator::Equal | BinaryOperator::NotEqual | BinaryOperator::Multiply => {
            fast_column_scalar(op, column, scalar)
        }
        BinaryOperator::GreaterThan => fast_column_scalar(BinaryOperator::LessThan, column, scalar),
        BinaryOperator::GreaterThanOrEqual => {
            fast_column_scalar(BinaryOperator::LessThanOrEqual, column, scalar)
        }
        BinaryOperator::LessThan => fast_column_scalar(BinaryOperator::GreaterThan, column, scalar),
        BinaryOperator::LessThanOrEqual => {
            fast_column_scalar(BinaryOperator::GreaterThanOrEqual, column, scalar)
        }
        BinaryOperator::Add => add_scalar_column(scalar, column),
        BinaryOperator::Subtract => sub_scalar_column(scalar, column),
        BinaryOperator::Divide => div_scalar_column(scalar, column),
        BinaryOperator::Modulo => mod_scalar_column(scalar, column),
        BinaryOperator::And | BinaryOperator::Or => Ok(Fast::Fallback),
    }
}

pub(crate) fn fast_columns(
    op: BinaryOperator,
    left: &ColumnArray,
    right: &ColumnArray,
) -> Result<Fast> {
    if left.len() != right.len() {
        return Err(anyhow!(
            "Column lengths {} and {} do not match",
            left.len(),
            right.len()
        ));
    }
    match (op, left, right) {
        (
            BinaryOperator::Add,
            ColumnArray::Utf8 {
                values: lhs,
                nulls: left_nulls,
            },
            ColumnArray::Utf8 {
                values: rhs,
                nulls: right_nulls,
            },
        ) => {
            let mut out = Vec::with_capacity(lhs.len());
            for index in 0..lhs.len() {
                if is_null_at(left_nulls, index) || is_null_at(right_nulls, index) {
                    return Err(anyhow!("Invalid arithmetic operands"));
                }
                let mut combined = String::with_capacity(lhs[index].len() + rhs[index].len());
                combined.push_str(&lhs[index]);
                combined.push_str(&rhs[index]);
                out.push(combined);
            }
            Ok(Fast::Done(column_value(ColumnArray::Utf8 {
                values: out,
                nulls: None,
            })))
        }
        _ => Ok(Fast::Fallback),
    }
}

fn eq_column_scalar(column: &ColumnArray, scalar: &Value, negate: bool) -> EvalValue {
    let values = match (column, scalar) {
        (ColumnArray::Null { len }, Value::Null) => vec![!negate; *len],
        (ColumnArray::Null { len }, _) => vec![negate; *len],
        (ColumnArray::Int64 { values, nulls }, Value::Int64(scalar)) => {
            cmp_eq(values, nulls, |value| value == scalar, negate)
        }
        (ColumnArray::Float64 { values, nulls }, Value::Float64(scalar)) => {
            cmp_eq(values, nulls, |value| value == scalar, negate)
        }
        (ColumnArray::Boolean { values, nulls }, Value::Boolean(scalar)) => {
            cmp_eq(values, nulls, |value| value == scalar, negate)
        }
        (ColumnArray::Utf8 { values, nulls }, Value::String(scalar)) => {
            cmp_eq(values, nulls, |value| value == scalar, negate)
        }
        (_, Value::Null) => (0..column.len())
            .map(|index| column.is_null(index) ^ negate)
            .collect(),
        _ => vec![negate; column.len()],
    };
    bool_column(values)
}

fn cmp_eq<T>(
    values: &[T],
    nulls: &Option<Vec<bool>>,
    mut pred: impl FnMut(&T) -> bool,
    negate: bool,
) -> Vec<bool> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            if is_null_at(nulls, index) {
                negate
            } else {
                pred(value) ^ negate
            }
        })
        .collect()
}

fn ord_column_scalar(op: BinaryOperator, column: &ColumnArray, scalar: &Value) -> Result<Fast> {
    if matches!(scalar, Value::Null) || matches!(column, ColumnArray::Null { .. }) {
        return Ok(Fast::Done(bool_column(vec![false; column.len()])));
    }
    let values = match (column, scalar) {
        (ColumnArray::Int64 { values, nulls }, Value::Int64(scalar)) => {
            ord_copy(values, nulls, |value| {
                ordering_matches(op, value.cmp(scalar))
            })
        }
        (ColumnArray::Float64 { values, nulls }, Value::Float64(scalar)) => {
            ord_f64(values, nulls, *scalar, op)?
        }
        (ColumnArray::Int64 { values, nulls }, Value::Float64(scalar)) => {
            let promoted: Vec<f64> = values.iter().map(|value| *value as f64).collect();
            ord_f64(&promoted, nulls, *scalar, op)?
        }
        (ColumnArray::Float64 { values, nulls }, Value::Int64(scalar)) => {
            ord_f64(values, nulls, *scalar as f64, op)?
        }
        (ColumnArray::Utf8 { values, nulls }, Value::String(scalar)) => {
            ord_copy(values, nulls, |value| {
                ordering_matches(op, value.cmp(scalar))
            })
        }
        (ColumnArray::Boolean { values, nulls }, Value::Boolean(scalar)) => {
            ord_copy(values, nulls, |value| {
                ordering_matches(op, value.cmp(scalar))
            })
        }
        _ => {
            return if all_null(column) {
                Ok(Fast::Done(bool_column(vec![false; column.len()])))
            } else {
                Err(anyhow!("Cannot compare different value types"))
            };
        }
    };
    Ok(Fast::Done(bool_column(values)))
}

fn all_null(column: &ColumnArray) -> bool {
    match column {
        ColumnArray::Null { .. } => true,
        ColumnArray::Int64 { nulls, .. }
        | ColumnArray::Float64 { nulls, .. }
        | ColumnArray::Boolean { nulls, .. }
        | ColumnArray::Utf8 { nulls, .. } => nulls.as_ref().is_some_and(|flags| {
            flags.len() == column.len() && flags.iter().all(|is_null| *is_null)
        }),
    }
}

fn ord_copy<T>(
    values: &[T],
    nulls: &Option<Vec<bool>>,
    mut pred: impl FnMut(&T) -> bool,
) -> Vec<bool> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            if is_null_at(nulls, index) {
                false
            } else {
                pred(value)
            }
        })
        .collect()
}

fn ord_f64(
    values: &[f64],
    nulls: &Option<Vec<bool>>,
    scalar: f64,
    op: BinaryOperator,
) -> Result<Vec<bool>> {
    let mut out = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        if is_null_at(nulls, index) {
            out.push(false);
            continue;
        }
        let ordering = value
            .partial_cmp(&scalar)
            .ok_or_else(|| anyhow!("Cannot compare NaN"))?;
        out.push(ordering_matches(op, ordering));
    }
    Ok(out)
}

fn ordering_matches(op: BinaryOperator, ordering: Ordering) -> bool {
    match op {
        BinaryOperator::GreaterThan => ordering.is_gt(),
        BinaryOperator::GreaterThanOrEqual => ordering.is_ge(),
        BinaryOperator::LessThan => ordering.is_lt(),
        BinaryOperator::LessThanOrEqual => ordering.is_le(),
        BinaryOperator::Equal
        | BinaryOperator::NotEqual
        | BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Multiply
        | BinaryOperator::Divide
        | BinaryOperator::Modulo
        | BinaryOperator::And
        | BinaryOperator::Or => false,
    }
}

fn arith_column_scalar(op: BinaryOperator, column: &ColumnArray, scalar: &Value) -> Result<Fast> {
    match (op, column, scalar) {
        (BinaryOperator::Add, ColumnArray::Utf8 { values, nulls }, Value::String(text)) => Ok(
            Fast::Done(column_value(concat_utf8(values, nulls, text, false)?)),
        ),
        (BinaryOperator::Add, ColumnArray::Int64 { values, nulls }, Value::Int64(scalar)) => {
            Ok(Fast::Done(column_value(map_i64(values, nulls, |value| {
                value.checked_add(*scalar)
            })?)))
        }
        (BinaryOperator::Subtract, ColumnArray::Int64 { values, nulls }, Value::Int64(scalar)) => {
            Ok(Fast::Done(column_value(map_i64(values, nulls, |value| {
                value.checked_sub(*scalar)
            })?)))
        }
        (BinaryOperator::Multiply, ColumnArray::Int64 { values, nulls }, Value::Int64(scalar)) => {
            Ok(Fast::Done(column_value(map_i64(values, nulls, |value| {
                value.checked_mul(*scalar)
            })?)))
        }
        (BinaryOperator::Divide, ColumnArray::Int64 { values, nulls }, Value::Int64(scalar)) => {
            Ok(Fast::Done(column_value(map_i64(values, nulls, |value| {
                value.checked_div(*scalar)
            })?)))
        }
        (BinaryOperator::Modulo, ColumnArray::Int64 { values, nulls }, Value::Int64(scalar)) => {
            Ok(Fast::Done(column_value(map_i64(values, nulls, |value| {
                value.checked_rem(*scalar)
            })?)))
        }
        (BinaryOperator::Add, ColumnArray::Float64 { values, nulls }, Value::Float64(scalar)) => {
            Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
                value + *scalar
            })?)))
        }
        (
            BinaryOperator::Subtract,
            ColumnArray::Float64 { values, nulls },
            Value::Float64(scalar),
        ) => Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
            value - *scalar
        })?))),
        (
            BinaryOperator::Multiply,
            ColumnArray::Float64 { values, nulls },
            Value::Float64(scalar),
        ) => Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
            value * *scalar
        })?))),
        (
            BinaryOperator::Divide,
            ColumnArray::Float64 { values, nulls },
            Value::Float64(scalar),
        ) => Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
            value / *scalar
        })?))),
        (BinaryOperator::Add, ColumnArray::Int64 { values, nulls }, Value::Float64(scalar)) => Ok(
            Fast::Done(column_value(map_f64_from_i64(values, nulls, |value| {
                value + *scalar
            })?)),
        ),
        (
            BinaryOperator::Subtract,
            ColumnArray::Int64 { values, nulls },
            Value::Float64(scalar),
        ) => Ok(Fast::Done(column_value(map_f64_from_i64(
            values,
            nulls,
            |value| value - *scalar,
        )?))),
        (
            BinaryOperator::Multiply,
            ColumnArray::Int64 { values, nulls },
            Value::Float64(scalar),
        ) => Ok(Fast::Done(column_value(map_f64_from_i64(
            values,
            nulls,
            |value| value * *scalar,
        )?))),
        (BinaryOperator::Divide, ColumnArray::Int64 { values, nulls }, Value::Float64(scalar)) => {
            Ok(Fast::Done(column_value(map_f64_from_i64(
                values,
                nulls,
                |value| value / *scalar,
            )?)))
        }
        (BinaryOperator::Add, ColumnArray::Float64 { values, nulls }, Value::Int64(scalar)) => {
            Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
                value + *scalar as f64
            })?)))
        }
        (
            BinaryOperator::Subtract,
            ColumnArray::Float64 { values, nulls },
            Value::Int64(scalar),
        ) => Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
            value - *scalar as f64
        })?))),
        (
            BinaryOperator::Multiply,
            ColumnArray::Float64 { values, nulls },
            Value::Int64(scalar),
        ) => Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
            value * *scalar as f64
        })?))),
        (BinaryOperator::Divide, ColumnArray::Float64 { values, nulls }, Value::Int64(scalar)) => {
            Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
                value / *scalar as f64
            })?)))
        }
        _ => Ok(Fast::Fallback),
    }
}

fn add_scalar_column(scalar: &Value, column: &ColumnArray) -> Result<Fast> {
    match (scalar, column) {
        (Value::String(text), ColumnArray::Utf8 { values, nulls }) => Ok(Fast::Done(column_value(
            concat_utf8(values, nulls, text, true)?,
        ))),
        (Value::String(_), _) | (_, ColumnArray::Utf8 { .. }) => Ok(Fast::Fallback),
        _ => arith_column_scalar(BinaryOperator::Add, column, scalar),
    }
}

fn sub_scalar_column(scalar: &Value, column: &ColumnArray) -> Result<Fast> {
    match (scalar, column) {
        (Value::Int64(scalar), ColumnArray::Int64 { values, nulls }) => {
            Ok(Fast::Done(column_value(map_i64(values, nulls, |value| {
                scalar.checked_sub(value)
            })?)))
        }
        (Value::Float64(scalar), ColumnArray::Float64 { values, nulls }) => {
            Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
                *scalar - value
            })?)))
        }
        (Value::Float64(scalar), ColumnArray::Int64 { values, nulls }) => Ok(Fast::Done(
            column_value(map_f64_from_i64(values, nulls, |value| *scalar - value)?),
        )),
        (Value::Int64(scalar), ColumnArray::Float64 { values, nulls }) => {
            Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
                *scalar as f64 - value
            })?)))
        }
        _ => Ok(Fast::Fallback),
    }
}

fn div_scalar_column(scalar: &Value, column: &ColumnArray) -> Result<Fast> {
    match (scalar, column) {
        (Value::Int64(scalar), ColumnArray::Int64 { values, nulls }) => {
            Ok(Fast::Done(column_value(map_i64(values, nulls, |value| {
                scalar.checked_div(value)
            })?)))
        }
        (Value::Float64(scalar), ColumnArray::Float64 { values, nulls }) => {
            Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
                *scalar / value
            })?)))
        }
        (Value::Float64(scalar), ColumnArray::Int64 { values, nulls }) => Ok(Fast::Done(
            column_value(map_f64_from_i64(values, nulls, |value| *scalar / value)?),
        )),
        (Value::Int64(scalar), ColumnArray::Float64 { values, nulls }) => {
            Ok(Fast::Done(column_value(map_f64(values, nulls, |value| {
                *scalar as f64 / value
            })?)))
        }
        _ => Ok(Fast::Fallback),
    }
}

fn mod_scalar_column(scalar: &Value, column: &ColumnArray) -> Result<Fast> {
    match (scalar, column) {
        (Value::Int64(scalar), ColumnArray::Int64 { values, nulls }) => {
            Ok(Fast::Done(column_value(map_i64(values, nulls, |value| {
                scalar.checked_rem(value)
            })?)))
        }
        _ => Ok(Fast::Fallback),
    }
}

fn concat_utf8(
    values: &[String],
    nulls: &Option<Vec<bool>>,
    scalar: &str,
    scalar_on_left: bool,
) -> Result<ColumnArray> {
    let mut out = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        if is_null_at(nulls, index) {
            return Err(anyhow!("Invalid arithmetic operands"));
        }
        let mut combined = String::with_capacity(value.len() + scalar.len());
        if scalar_on_left {
            combined.push_str(scalar);
            combined.push_str(value);
        } else {
            combined.push_str(value);
            combined.push_str(scalar);
        }
        out.push(combined);
    }
    Ok(ColumnArray::Utf8 {
        values: out,
        nulls: None,
    })
}

fn map_i64(
    values: &[i64],
    nulls: &Option<Vec<bool>>,
    op: impl Fn(i64) -> Option<i64>,
) -> Result<ColumnArray> {
    let mut out = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        if is_null_at(nulls, index) {
            return Err(anyhow!("Invalid arithmetic operands"));
        }
        out.push(
            op(*value).ok_or_else(|| anyhow!("Integer arithmetic overflow or division by zero"))?,
        );
    }
    Ok(ColumnArray::Int64 {
        values: out,
        nulls: None,
    })
}

fn map_f64(
    values: &[f64],
    nulls: &Option<Vec<bool>>,
    op: impl Fn(f64) -> f64,
) -> Result<ColumnArray> {
    let mut out = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        if is_null_at(nulls, index) {
            return Err(anyhow!("Invalid arithmetic operands"));
        }
        out.push(finite(op(*value))?);
    }
    Ok(ColumnArray::Float64 {
        values: out,
        nulls: None,
    })
}

fn map_f64_from_i64(
    values: &[i64],
    nulls: &Option<Vec<bool>>,
    op: impl Fn(f64) -> f64,
) -> Result<ColumnArray> {
    let mut out = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        if is_null_at(nulls, index) {
            return Err(anyhow!("Invalid arithmetic operands"));
        }
        out.push(finite(op(*value as f64))?);
    }
    Ok(ColumnArray::Float64 {
        values: out,
        nulls: None,
    })
}

fn finite(value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(anyhow!(
            "Floating-point arithmetic produced a non-finite result"
        ))
    }
}

pub(crate) fn apply_not(value: &EvalValue) -> Result<Fast> {
    match value {
        EvalValue::Column(column) => match column.as_ref() {
            ColumnArray::Boolean { values, nulls } => {
                if nulls
                    .as_ref()
                    .is_some_and(|flags| flags.iter().any(|is_null| *is_null))
                {
                    Err(anyhow!("Invalid unary operation"))
                } else {
                    Ok(Fast::Done(bool_column(
                        values.iter().map(|flag| !flag).collect(),
                    )))
                }
            }
            ColumnArray::Null { .. } => Err(anyhow!("Invalid unary operation")),
            _ => Ok(Fast::Fallback),
        },
        EvalValue::Scalar(_) => Ok(Fast::Fallback),
    }
}

pub(crate) fn apply_negate(value: &EvalValue) -> Result<Fast> {
    match value {
        EvalValue::Column(column) => match column.as_ref() {
            ColumnArray::Int64 { values, nulls } => {
                let mut out = Vec::with_capacity(values.len());
                for (index, number) in values.iter().enumerate() {
                    if is_null_at(nulls, index) {
                        return Err(anyhow!("Invalid unary operation"));
                    }
                    out.push(number.checked_neg().ok_or_else(|| {
                        anyhow!("Integer arithmetic overflow or division by zero")
                    })?);
                }
                Ok(Fast::Done(column_value(ColumnArray::Int64 {
                    values: out,
                    nulls: None,
                })))
            }
            ColumnArray::Float64 { values, nulls } => {
                let mut out = Vec::with_capacity(values.len());
                for (index, number) in values.iter().enumerate() {
                    if is_null_at(nulls, index) {
                        return Err(anyhow!("Invalid unary operation"));
                    }
                    out.push(finite(-number)?);
                }
                Ok(Fast::Done(column_value(ColumnArray::Float64 {
                    values: out,
                    nulls: None,
                })))
            }
            ColumnArray::Null { .. } => Err(anyhow!("Invalid unary operation")),
            _ => Ok(Fast::Fallback),
        },
        EvalValue::Scalar(_) => Ok(Fast::Fallback),
    }
}

pub(crate) fn map_utf8(
    column: &ColumnArray,
    message: &'static str,
    map: impl Fn(&str) -> String,
) -> Result<Fast> {
    match column {
        ColumnArray::Utf8 { values, nulls } => {
            let mut out = Vec::with_capacity(values.len());
            for (index, value) in values.iter().enumerate() {
                if is_null_at(nulls, index) {
                    return Err(anyhow!(message));
                }
                out.push(map(value));
            }
            Ok(Fast::Done(column_value(ColumnArray::Utf8 {
                values: out,
                nulls: None,
            })))
        }
        ColumnArray::Null { .. } => Err(anyhow!(message)),
        _ => Ok(Fast::Fallback),
    }
}
