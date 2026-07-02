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
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>> {
        let mut reader = csv::Reader::from_path(&self.path)?;
        let headers = reader.headers()?.clone();

        let iter = reader.into_records().map(move |row| {
            let row = row.map_err(anyhow::Error::from)?;
            let mut record = Record::new();
            for (header, value) in headers.iter().zip(row.iter()) {
                record.insert(header, Value::string(value));
            }
            Ok(record)
        });

        Ok(Box::new(iter))
    }
}
