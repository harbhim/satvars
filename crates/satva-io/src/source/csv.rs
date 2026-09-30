use anyhow::Result;
use csv;
use satva_core::source::Source;
use satva_types::{Record, Value};
use std::path::PathBuf;

pub struct CsvSource {
    path: PathBuf,
    delimiter: u8,
}

impl CsvSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_delimiter(path, b',')
    }

    pub fn with_delimiter(path: impl Into<PathBuf>, delimiter: u8) -> Self {
        Self {
            path: path.into(),
            delimiter,
        }
    }
}

impl Source for CsvSource {
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(self.delimiter)
            .from_path(&self.path)?;
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
