use anyhow::Result;
use csv;
use satva_core::sink::Sink;
use satva_types::Record;
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

        let headers = self
            .headers
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Headers not initialized"))?;

        if self.writer.is_none() {
            let mut writer = csv::Writer::from_path(&self.path)?;
            writer.write_record(headers)?;
            self.writer = Some(writer);
        }

        let row = headers
            .iter()
            .map(|header| {
                record
                    .fields
                    .get(header)
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Writer not initialized"))?;
        writer.write_record(row)?;

        Ok(())
    }
}
