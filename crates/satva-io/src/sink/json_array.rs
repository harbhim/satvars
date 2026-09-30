use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use satva_core::sink::Sink;
use satva_types::Record;

use super::json::to_json_value;

/// Writes one JSON array. The file is valid JSON after [`Sink::finish`].
pub struct JsonArraySink {
    path: PathBuf,
    writer: Option<BufWriter<File>>,
    started: bool,
    finished: bool,
}

impl JsonArraySink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            writer: None,
            started: false,
            finished: false,
        }
    }

    fn writer(&mut self) -> Result<&mut BufWriter<File>> {
        if self.writer.is_none() {
            let file = File::create(&self.path)
                .with_context(|| format!("Failed to create {}", self.path.display()))?;
            self.writer = Some(BufWriter::new(file));
        }

        self.writer
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("JSON array writer is not open"))
    }
}

impl Sink for JsonArraySink {
    fn finish(&mut self) -> Result<()> {
        if self.finished {
            return Ok(());
        }
        let started = self.started;
        {
            let writer = self.writer()?;
            if started {
                writer.write_all(b"\n]\n")?;
            } else {
                writer.write_all(b"[]\n")?;
            }
            writer.flush()?;
        }
        self.finished = true;
        Ok(())
    }

    fn write(&mut self, record: &Record) -> Result<()> {
        if self.finished {
            return Err(anyhow::anyhow!(
                "Cannot write to a finished JSON array sink"
            ));
        }

        let mut object = serde_json::Map::new();
        for (key, value) in record {
            object.insert(key.clone(), to_json_value(value));
        }

        let started = self.started;
        {
            let writer = self.writer()?;
            if started {
                writer.write_all(b",\n")?;
            } else {
                writer.write_all(b"[\n")?;
            }
            serde_json::to_writer(&mut *writer, &serde_json::Value::Object(object))?;
        }
        self.started = true;
        Ok(())
    }
}
