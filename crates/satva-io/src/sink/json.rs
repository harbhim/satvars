use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::Result;
use satva_core::{record::Record, sink::Sink, value::Value};

pub struct JsonSink {
    path: PathBuf,
    writer: Option<BufWriter<File>>,
}

impl JsonSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            writer: None,
        }
    }

    fn writer(&mut self) -> Result<&mut BufWriter<File>> {
        if self.writer.is_none() {
            let file = File::create(&self.path)?;
            self.writer = Some(BufWriter::new(file));
        }

        Ok(self.writer.as_mut().unwrap())
    }
}

impl Sink for JsonSink {
    fn write(&mut self, record: &Record) -> Result<()> {
        let mut object = serde_json::Map::new();

        for (key, value) in &record.fields {
            object.insert(key.clone(), to_json_value(value));
        }

        let json = serde_json::Value::Object(object);

        let writer = self.writer()?;

        serde_json::to_writer(&mut *writer, &json)?;
        writer.write_all(b"\n")?;

        Ok(())
    }
}

fn to_json_value(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,

        Value::Boolean(v) => serde_json::Value::Bool(*v),

        Value::Int64(v) => serde_json::Value::Number((*v).into()),

        Value::Float64(v) => serde_json::Number::from_f64(*v)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),

        Value::String(v) => serde_json::Value::String(v.clone()),
    }
}
