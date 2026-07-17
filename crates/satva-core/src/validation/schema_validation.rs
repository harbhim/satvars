use crate::pipeline::PipelineStage;
use crate::pipeline::StageContext;
use crate::pipeline::StageError;
use crate::pipeline::StageResult;
use satva_types::{DataType, Record, Schema, Value};

pub struct SchemaValidation {
    schema: Schema,
}

impl SchemaValidation {
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }

    fn coerce_value(
        &self,
        value: &Value,
        data_type: DataType,
        is_nullable: bool,
        key: &str,
    ) -> Result<Value, StageError> {
        match data_type {
            DataType::Null => {
                if *value != Value::Null {
                    return Err(StageError::execution(
                        self.name(),
                        format!("Field '{key}' must be null"),
                    ));
                }
                Ok(Value::Null)
            }
            DataType::String => {
                let coerced = if let Value::String(s) = value {
                    s.clone()
                } else {
                    value.to_string()
                };
                if coerced.is_empty() && !is_nullable {
                    return Err(StageError::execution(
                        self.name(),
                        format!("Field '{key}' is not nullable but got empty string"),
                    ));
                }
                Ok(Value::String(coerced))
            }
            DataType::Int64 => self.coerce_int64(value, is_nullable, key),
            DataType::Float64 => self.coerce_float64(value, is_nullable, key),
            DataType::Boolean => self.coerce_boolean(value, is_nullable, key),
        }
    }

    fn coerce_int64(
        &self,
        value: &Value,
        is_nullable: bool,
        key: &str,
    ) -> Result<Value, StageError> {
        match value {
            Value::Int64(v) => Ok(Value::Int64(*v)),
            Value::String(s) => {
                if s.is_empty() {
                    if is_nullable {
                        Ok(Value::Null)
                    } else {
                        Err(StageError::execution(
                            self.name(),
                            format!("Field '{key}' is not nullable but got empty string"),
                        ))
                    }
                } else {
                    s.parse::<i64>().map(Value::Int64).map_err(|e| {
                        StageError::execution(
                            self.name(),
                            format!("Failed to parse field '{key}' with value '{s}' as Int64: {e}"),
                        )
                    })
                }
            }
            _ => Err(StageError::execution(
                self.name(),
                format!("Field '{key}' is not compatible with Int64"),
            )),
        }
    }

    fn coerce_float64(
        &self,
        value: &Value,
        is_nullable: bool,
        key: &str,
    ) -> Result<Value, StageError> {
        match value {
            Value::Float64(v) => Ok(Value::Float64(*v)),
            Value::Int64(v) => Ok(Value::Float64(*v as f64)),
            Value::String(s) => {
                if s.is_empty() {
                    if is_nullable {
                        Ok(Value::Null)
                    } else {
                        Err(StageError::execution(
                            self.name(),
                            format!("Field '{key}' is not nullable but got empty string"),
                        ))
                    }
                } else {
                    s.parse::<f64>().map(Value::Float64).map_err(|e| {
                        StageError::execution(
                            self.name(),
                            format!(
                                "Failed to parse field '{key}' with value '{s}' as Float64: {e}"
                            ),
                        )
                    })
                }
            }
            _ => Err(StageError::execution(
                self.name(),
                format!("Field '{key}' is not compatible with Float64"),
            )),
        }
    }

    fn coerce_boolean(
        &self,
        value: &Value,
        is_nullable: bool,
        key: &str,
    ) -> Result<Value, StageError> {
        match value {
            Value::Boolean(v) => Ok(Value::Boolean(*v)),
            Value::String(s) => {
                if s.is_empty() {
                    if is_nullable {
                        Ok(Value::Null)
                    } else {
                        Err(StageError::execution(
                            self.name(),
                            format!("Field '{key}' is not nullable but got empty string"),
                        ))
                    }
                } else {
                    s.parse::<bool>().map(Value::Boolean).map_err(|e| {
                        StageError::execution(
                            self.name(),
                            format!(
                                "Failed to parse field '{key}' with value '{s}' as Boolean: {e}"
                            ),
                        )
                    })
                }
            }
            _ => Err(StageError::execution(
                self.name(),
                format!("Field '{key}' is not compatible with Boolean"),
            )),
        }
    }
}

impl PipelineStage for SchemaValidation {
    fn name(&self) -> &'static str {
        "SchemaValidation"
    }

    fn execute(&self, record: &mut Record, _ctx: &StageContext) -> StageResult {
        for field in self.schema.fields() {
            let key = field.name();
            match record.get(key) {
                None | Some(Value::Null) => {
                    if !field.is_nullable() {
                        return StageResult::Fail {
                            error: StageError::execution(
                                self.name(),
                                format!("Missing required field: {key}"),
                            ),
                        };
                    }
                    record.insert(key, Value::Null);
                }
                Some(value) => {
                    match self.coerce_value(value, field.data_type(), field.is_nullable(), key) {
                        Ok(coerced_value) => {
                            record.insert(key, coerced_value);
                        }
                        Err(error) => return StageResult::Fail { error },
                    }
                }
            }
        }
        StageResult::Continue
    }
}
