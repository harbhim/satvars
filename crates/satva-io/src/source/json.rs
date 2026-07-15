use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use satva_core::source::Source;
use satva_types::{Record, Value};

pub struct JsonSource {
    path: PathBuf,
}

impl JsonSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl Source for JsonSource {
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        let iter = reader.lines().map(|line| {
            let line = line?;

            let json: serde_json::Value = serde_json::from_str(&line)?;

            let object = json
                .as_object()
                .ok_or_else(|| anyhow!("Each JSON line must contain an object"))?;

            let mut record = Record::new();

            for (key, value) in object {
                record.insert(key, json_value_to_value(value));
            }

            Ok(record)
        });

        Ok(Box::new(iter))
    }
}

fn json_value_to_value(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,

        serde_json::Value::Bool(v) => Value::Boolean(*v),

        serde_json::Value::Number(v) => {
            if let Some(i) = v.as_i64() {
                Value::Int64(i)
            } else {
                Value::Float64(v.as_f64().unwrap())
            }
        }

        serde_json::Value::String(v) => Value::String(v.clone()),

        // Nested structures will become JSON strings for now.
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            Value::String(value.to_string())
        }
    }
}
