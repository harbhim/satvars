use anyhow::Result;
use csv;
use satva_core::{record::Record, source::Source, value::Value};
use std::path::PathBuf;

pub struct CsvSource {
    path: PathBuf,
}

impl CsvSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl Source for CsvSource {
    fn read(&self) -> Result<Vec<Record>> {
        let mut reader = csv::Reader::from_path(&self.path)?;
        let headers = reader.headers()?.clone();
        let mut records = Vec::new();
        for row in reader.records() {
            let row = row?;
            let mut record = Record::new();
            for (header, value) in headers.iter().zip(row.iter()) {
                record.insert(header, Value::string(value));
            }
            records.push(record);
        }
        Ok(records)
    }
}
