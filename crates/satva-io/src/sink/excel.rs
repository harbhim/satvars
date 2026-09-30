use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use rust_xlsxwriter::Workbook;
use satva_core::sink::Sink;
use satva_types::{Record, Value};

/// Writes an `.xlsx` workbook. Rows are buffered until [`Sink::finish`].
///
/// Columns come from the first record. Missing fields are blank. Extra fields are dropped.
pub struct ExcelSink {
    path: PathBuf,
    sheet: String,
    headers: Vec<String>,
    records: Vec<Record>,
    started: bool,
}

impl ExcelSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_sheet(path, "Sheet1")
    }

    pub fn with_sheet(path: impl Into<PathBuf>, sheet: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            sheet: sheet.into(),
            headers: Vec::new(),
            records: Vec::new(),
            started: false,
        }
    }
}

impl Sink for ExcelSink {
    fn finish(&mut self) -> Result<()> {
        if !self.started {
            return Ok(());
        }

        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(&self.sheet)
            .with_context(|| format!("Failed to name Excel worksheet '{}'", self.sheet))?;

        for (column, header) in self.headers.iter().enumerate() {
            let column = u16::try_from(column).context("Excel column index exceeds u16")?;
            worksheet.write_string(0, column, header)?;
        }

        for (row_index, record) in self.records.iter().enumerate() {
            let row = u32::try_from(row_index + 1).context("Excel row index exceeds u32")?;
            for (column, header) in self.headers.iter().enumerate() {
                let column = u16::try_from(column).context("Excel column index exceeds u16")?;
                match record.get(header).unwrap_or(&Value::Null) {
                    Value::Null => {}
                    Value::Int64(value) => {
                        worksheet.write(row, column, *value)?;
                    }
                    Value::Float64(value) => {
                        worksheet.write(row, column, *value)?;
                    }
                    Value::Boolean(value) => {
                        worksheet.write(row, column, *value)?;
                    }
                    Value::String(value) => {
                        worksheet.write(row, column, value)?;
                    }
                }
            }
        }

        workbook
            .save(&self.path)
            .with_context(|| format!("Failed to save workbook {}", self.path.display()))?;
        self.records.clear();
        self.started = false;
        Ok(())
    }

    fn write(&mut self, record: &Record) -> Result<()> {
        if !self.started {
            if record.is_empty() {
                return Err(anyhow!(
                    "Excel sink cannot infer columns from an empty record"
                ));
            }
            self.headers = record.keys().cloned().collect();
            self.started = true;
        }
        self.records.push(record.clone());
        Ok(())
    }
}
