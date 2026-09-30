use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use satva_core::source::Source;
use satva_types::Record;

use super::json::json_value_to_value;

/// Reads a JSON file whose top-level value is an array of objects.
pub struct JsonArraySource {
    path: PathBuf,
}

impl JsonArraySource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl Source for JsonArraySource {
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>> {
        let file = File::open(&self.path)
            .with_context(|| format!("Failed to open {}", self.path.display()))?;
        let json: serde_json::Value = serde_json::from_reader(BufReader::new(file))
            .with_context(|| format!("Failed to parse JSON array {}", self.path.display()))?;

        let items = json.as_array().ok_or_else(|| {
            anyhow!(
                "JSON array source {} requires a top-level array",
                self.path.display()
            )
        })?;

        let records = items
            .iter()
            .map(|item| {
                let object = item
                    .as_object()
                    .ok_or_else(|| anyhow!("Each JSON array element must be an object"))?;
                let mut record = Record::new();
                for (key, value) in object {
                    record.insert(key, json_value_to_value(value));
                }
                Ok(record)
            })
            .collect::<Vec<_>>();

        Ok(Box::new(records.into_iter()))
    }
}
