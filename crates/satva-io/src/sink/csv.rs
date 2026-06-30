use anyhow::Result;
use csv;
use satva_core::{record::Record, sink::Sink};
use std::fs::File;
use std::path::PathBuf;

pub struct CsvSink {
    path: PathBuf,
    writer: Option<csv::Writer<File>>,
    headers: Option<Vec<String>>,
}

impl CsvSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            writer: None,
            headers: None,
        }
    }
}

impl Sink for CsvSink {
    fn write(&mut self, record: &Record) -> Result<()> {
        if self.headers.is_none() {
            let mut headers = record.fields.keys().cloned().collect::<Vec<_>>();
            headers.sort();
            self.headers = Some(headers);
        }

        if self.writer.is_none() {
            let mut writer = csv::Writer::from_path(&self.path)?;
            writer.write_record(self.headers.as_ref().unwrap())?;
            self.writer = Some(writer);
        }

        let headers = self.headers.as_ref().unwrap();
        let row = headers
            .iter()
            .map(|header| {
                record
                    .fields
                    .get(header)
                    .map(|value| value.to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        self.writer.as_mut().unwrap().write_record(row)?;

        Ok(())
    }
}
