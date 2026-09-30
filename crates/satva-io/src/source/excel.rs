use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use calamine::{Data, Reader, open_workbook_auto};
use satva_core::source::Source;
use satva_types::{Record, Value};

/// Reads the first row as headers and the remaining rows as records.
///
/// When `sheet` is omitted, the first worksheet is used.
pub struct ExcelSource {
    path: PathBuf,
    sheet: Option<String>,
}

impl ExcelSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            sheet: None,
        }
    }

    pub fn with_sheet(path: impl Into<PathBuf>, sheet: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            sheet: Some(sheet.into()),
        }
    }
}

impl Source for ExcelSource {
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>> {
        let mut workbook = open_workbook_auto(&self.path)
            .with_context(|| format!("Failed to open workbook {}", self.path.display()))?;

        let sheet_name = match &self.sheet {
            Some(name) => name.clone(),
            None => workbook
                .sheet_names()
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("Workbook {} has no worksheets", self.path.display()))?,
        };

        let range = workbook
            .worksheet_range(&sheet_name)
            .with_context(|| format!("Failed to read worksheet '{sheet_name}'"))?;

        let mut rows = range.rows();
        let Some(header_row) = rows.next() else {
            return Ok(Box::new(std::iter::empty()));
        };

        let headers = header_row
            .iter()
            .enumerate()
            .map(|(index, cell)| header_name(cell, index))
            .collect::<Result<Vec<_>>>()?;

        let records = rows
            .map(|row| {
                let mut record = Record::new();
                for (index, header) in headers.iter().enumerate() {
                    let value = row
                        .get(index)
                        .map(cell_value)
                        .transpose()?
                        .unwrap_or(Value::Null);
                    record.insert(header, value);
                }
                Ok(record)
            })
            .collect::<Vec<_>>();

        Ok(Box::new(records.into_iter()))
    }
}

fn header_name(cell: &Data, index: usize) -> Result<String> {
    let name = match cell {
        Data::String(value) => value.trim().to_string(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTimeIso(value) | Data::DurationIso(value) => value.clone(),
        Data::DateTime(value) => datetime_string(value)?,
        Data::Empty => String::new(),
        Data::Error(error) => {
            return Err(anyhow!("Header cell {index} is an Excel error: {error:?}"));
        }
    };

    if name.is_empty() {
        return Err(anyhow!("Header cell {index} is empty"));
    }
    Ok(name)
}

fn cell_value(cell: &Data) -> Result<Value> {
    match cell {
        Data::Empty => Ok(Value::Null),
        Data::String(value) | Data::DateTimeIso(value) | Data::DurationIso(value) => {
            Ok(Value::String(value.clone()))
        }
        Data::Int(value) => Ok(Value::Int64(*value)),
        Data::Float(value) => Ok(float_cell(*value)),
        Data::Bool(value) => Ok(Value::Boolean(*value)),
        Data::DateTime(value) => Ok(Value::String(datetime_string(value)?)),
        Data::Error(error) => Err(anyhow!("Excel cell error: {error:?}")),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::float_cmp)]
fn float_cell(value: f64) -> Value {
    // Excel stores every number as f64. Integers up to 2^53 are exact.
    const EXACT_INT_LIMIT: f64 = 9_007_199_254_740_992.0;
    if value.is_finite() && value.fract() == 0.0 && value.abs() <= EXACT_INT_LIMIT {
        Value::Int64(value as i64)
    } else {
        Value::Float64(value)
    }
}

fn datetime_string(value: &calamine::ExcelDateTime) -> Result<String> {
    value
        .as_datetime()
        .map(|datetime| datetime.to_string())
        .ok_or_else(|| anyhow!("Excel datetime {value:?} is out of range"))
}
