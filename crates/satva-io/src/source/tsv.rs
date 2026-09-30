use std::path::PathBuf;

use anyhow::Result;
use satva_core::source::Source;
use satva_types::Record;

use super::csv::CsvSource;

/// Tab-separated values with a header row. Cells are strings, same as [`CsvSource`].
pub struct TsvSource {
    inner: CsvSource,
}

impl TsvSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            inner: CsvSource::with_delimiter(path, b'\t'),
        }
    }
}

impl Source for TsvSource {
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>> {
        self.inner.read()
    }
}
