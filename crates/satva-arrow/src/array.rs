use anyhow::{Result, anyhow};
use satva_types::Value;

/// Columnar values for one field. `nulls[i] == true` means row `i` is null.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnArray {
    Null {
        len: usize,
    },
    Int64 {
        values: Vec<i64>,
        nulls: Option<Vec<bool>>,
    },
    Float64 {
        values: Vec<f64>,
        nulls: Option<Vec<bool>>,
    },
    Boolean {
        values: Vec<bool>,
        nulls: Option<Vec<bool>>,
    },
    Utf8 {
        values: Vec<String>,
        nulls: Option<Vec<bool>>,
    },
}

impl ColumnArray {
    pub fn len(&self) -> usize {
        match self {
            Self::Null { len } => *len,
            Self::Int64 { values, .. } => values.len(),
            Self::Float64 { values, .. } => values.len(),
            Self::Boolean { values, .. } => values.len(),
            Self::Utf8 { values, .. } => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn is_null(&self, index: usize) -> bool {
        match self {
            Self::Null { .. } => true,
            Self::Int64 { nulls, .. }
            | Self::Float64 { nulls, .. }
            | Self::Boolean { nulls, .. }
            | Self::Utf8 { nulls, .. } => is_null_at(nulls, index),
        }
    }

    pub(crate) fn value_at(&self, index: usize) -> Value {
        if self.is_null(index) {
            return Value::Null;
        }
        match self {
            Self::Null { .. } => Value::Null,
            Self::Int64 { values, .. } => Value::Int64(values[index]),
            Self::Float64 { values, .. } => Value::Float64(values[index]),
            Self::Boolean { values, .. } => Value::Boolean(values[index]),
            Self::Utf8 { values, .. } => Value::String(values[index].clone()),
        }
    }

    pub(crate) fn broadcast(value: &Value, len: usize) -> Self {
        match value {
            Value::Null => Self::Null { len },
            Value::Int64(number) => Self::Int64 {
                values: vec![*number; len],
                nulls: None,
            },
            Value::Float64(number) => Self::Float64 {
                values: vec![*number; len],
                nulls: None,
            },
            Value::Boolean(flag) => Self::Boolean {
                values: vec![*flag; len],
                nulls: None,
            },
            Value::String(text) => Self::Utf8 {
                values: vec![text.clone(); len],
                nulls: None,
            },
        }
    }

    pub(crate) fn empty_like(&self) -> Self {
        match self {
            Self::Null { .. } => Self::Null { len: 0 },
            Self::Int64 { .. } => Self::Int64 {
                values: Vec::new(),
                nulls: None,
            },
            Self::Float64 { .. } => Self::Float64 {
                values: Vec::new(),
                nulls: None,
            },
            Self::Boolean { .. } => Self::Boolean {
                values: Vec::new(),
                nulls: None,
            },
            Self::Utf8 { .. } => Self::Utf8 {
                values: Vec::new(),
                nulls: None,
            },
        }
    }

    pub(crate) fn take(&self, indices: &[usize]) -> Self {
        match self {
            Self::Null { .. } => Self::Null { len: indices.len() },
            Self::Int64 { values, nulls } => Self::Int64 {
                values: gather_copy(values, indices),
                nulls: take_nulls(nulls.as_deref(), indices),
            },
            Self::Float64 { values, nulls } => Self::Float64 {
                values: gather_copy(values, indices),
                nulls: take_nulls(nulls.as_deref(), indices),
            },
            Self::Boolean { values, nulls } => Self::Boolean {
                values: gather_copy(values, indices),
                nulls: take_nulls(nulls.as_deref(), indices),
            },
            Self::Utf8 { values, nulls } => Self::Utf8 {
                values: gather_clone(values, indices),
                nulls: take_nulls(nulls.as_deref(), indices),
            },
        }
    }

    pub(crate) fn from_values(values: &[Value]) -> Result<Self> {
        let Some(prototype) = values.iter().find(|value| !matches!(value, Value::Null)) else {
            return Ok(Self::Null { len: values.len() });
        };
        match prototype {
            Value::Boolean(_) => bool_column(values),
            Value::String(_) => utf8_column(values),
            Value::Int64(_) | Value::Float64(_) => numeric_column(values),
            Value::Null => unreachable!("prototype is non-null"),
        }
    }
}

#[allow(clippy::ref_option)]
pub(crate) fn is_null_at(nulls: &Option<Vec<bool>>, index: usize) -> bool {
    nulls.as_ref().is_some_and(|flags| flags[index])
}

pub(crate) fn pack_nulls(nulls: Vec<bool>) -> Option<Vec<bool>> {
    if nulls.contains(&true) {
        Some(nulls)
    } else {
        None
    }
}

fn take_nulls(nulls: Option<&[bool]>, indices: &[usize]) -> Option<Vec<bool>> {
    let nulls = nulls?;
    pack_nulls(indices.iter().map(|index| nulls[*index]).collect())
}

fn gather_copy<T: Copy>(values: &[T], indices: &[usize]) -> Vec<T> {
    indices.iter().map(|index| values[*index]).collect()
}

fn gather_clone<T: Clone>(values: &[T], indices: &[usize]) -> Vec<T> {
    indices.iter().map(|index| values[*index].clone()).collect()
}

fn bool_column(values: &[Value]) -> Result<ColumnArray> {
    let mut out = Vec::with_capacity(values.len());
    let mut nulls = Vec::with_capacity(values.len());
    for value in values {
        match value {
            Value::Null => {
                out.push(false);
                nulls.push(true);
            }
            Value::Boolean(flag) => {
                out.push(*flag);
                nulls.push(false);
            }
            _ => return Err(anyhow!("Cannot store mixed types in a column")),
        }
    }
    Ok(ColumnArray::Boolean {
        values: out,
        nulls: pack_nulls(nulls),
    })
}

fn utf8_column(values: &[Value]) -> Result<ColumnArray> {
    let mut out = Vec::with_capacity(values.len());
    let mut nulls = Vec::with_capacity(values.len());
    for value in values {
        match value {
            Value::Null => {
                out.push(String::new());
                nulls.push(true);
            }
            Value::String(text) => {
                out.push(text.clone());
                nulls.push(false);
            }
            _ => return Err(anyhow!("Cannot store mixed types in a column")),
        }
    }
    Ok(ColumnArray::Utf8 {
        values: out,
        nulls: pack_nulls(nulls),
    })
}

fn numeric_column(values: &[Value]) -> Result<ColumnArray> {
    let mut promote = false;
    for value in values {
        match value {
            Value::Null | Value::Int64(_) => {}
            Value::Float64(_) => promote = true,
            _ => return Err(anyhow!("Cannot store mixed types in a column")),
        }
    }
    let mut nulls = Vec::with_capacity(values.len());
    if promote {
        let mut out = Vec::with_capacity(values.len());
        for value in values {
            match value {
                Value::Null => {
                    out.push(0.0);
                    nulls.push(true);
                }
                Value::Int64(number) => {
                    out.push(*number as f64);
                    nulls.push(false);
                }
                Value::Float64(number) => {
                    out.push(*number);
                    nulls.push(false);
                }
                Value::Boolean(_) | Value::String(_) => {
                    return Err(anyhow!("Cannot store mixed types in a column"));
                }
            }
        }
        return Ok(ColumnArray::Float64 {
            values: out,
            nulls: pack_nulls(nulls),
        });
    }

    let mut out = Vec::with_capacity(values.len());
    for value in values {
        match value {
            Value::Null => {
                out.push(0);
                nulls.push(true);
            }
            Value::Int64(number) => {
                out.push(*number);
                nulls.push(false);
            }
            Value::Float64(_) | Value::Boolean(_) | Value::String(_) => {
                return Err(anyhow!("Cannot store mixed types in a column"));
            }
        }
    }
    Ok(ColumnArray::Int64 {
        values: out,
        nulls: pack_nulls(nulls),
    })
}
