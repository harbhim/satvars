use std::path::PathBuf;

use anyhow::Result;
use satva_core::sink::Sink;
use satva_types::Record;

use super::csv::CsvSink;

/// Tab-separated values with a header row taken from the first record.
pub struct TsvSink {
    inner: CsvSink,
}

impl TsvSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            inner: CsvSink::with_delimiter(path, b'\t'),
        }
    }
}

impl Sink for TsvSink {
    fn finish(&mut self) -> Result<()> {
        self.inner.finish()
    }

    fn write(&mut self, record: &Record) -> Result<()> {
        self.inner.write(record)
    }
}
