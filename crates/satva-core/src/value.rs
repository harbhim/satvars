#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int64(i64),
    Float64(f64),
    Boolean(bool),
    String(String),
}

impl Value {
    pub fn string(value: &str) -> Self {
        Self::String(value.to_string())
    }

    pub fn int64(value: i64) -> Self {
        Self::Int64(value)
    }

    pub fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int64(value) => Some(*value),
            _ => None,
        }
    }
}
