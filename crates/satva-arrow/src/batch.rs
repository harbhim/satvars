use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use satva_types::{Record, Value};

use crate::array::ColumnArray;

/// Default number of rows packed into one batch.
pub const DEFAULT_BATCH_SIZE: usize = 8_192;

/// A named column. Cloning clones the `Arc`, not the row data.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    name: String,
    data: Arc<ColumnArray>,
}

impl Column {
    pub fn new(name: impl Into<String>, data: ColumnArray) -> Self {
        Self::from_shared(name, Arc::new(data))
    }

    pub fn from_shared(name: impl Into<String>, data: Arc<ColumnArray>) -> Self {
        Self {
            name: name.into(),
            data,
        }
    }

    pub fn int64(name: impl Into<String>, values: Vec<i64>) -> Self {
        Self::new(
            name,
            ColumnArray::Int64 {
                values,
                nulls: None,
            },
        )
    }

    pub fn float64(name: impl Into<String>, values: Vec<f64>) -> Self {
        Self::new(
            name,
            ColumnArray::Float64 {
                values,
                nulls: None,
            },
        )
    }

    pub fn boolean(name: impl Into<String>, values: Vec<bool>) -> Self {
        Self::new(
            name,
            ColumnArray::Boolean {
                values,
                nulls: None,
            },
        )
    }

    pub fn utf8(name: impl Into<String>, values: Vec<String>) -> Self {
        Self::new(
            name,
            ColumnArray::Utf8 {
                values,
                nulls: None,
            },
        )
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data(&self) -> &Arc<ColumnArray> {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Columnar batch used as the unit of execution.
///
/// Cloning a batch increments column reference counts and does not copy rows.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordBatch {
    columns: Arc<[Column]>,
    num_rows: usize,
}

impl RecordBatch {
    pub fn try_new(columns: Vec<Column>) -> Result<Self> {
        let num_rows = columns.first().map_or(0, Column::len);
        Self::try_with_rows(columns, num_rows)
    }

    pub fn try_with_rows(columns: Vec<Column>, num_rows: usize) -> Result<Self> {
        if columns.iter().any(|column| column.len() != num_rows) {
            return Err(anyhow!("Column lengths do not match"));
        }
        let mut seen = HashSet::new();
        for column in &columns {
            if !seen.insert(column.name()) {
                return Err(anyhow!("Duplicate column '{}'", column.name()));
            }
        }
        Ok(Self {
            columns: Arc::from(columns),
            num_rows,
        })
    }

    pub fn from_records(records: &[Record]) -> Result<Self> {
        if records.is_empty() {
            return Self::try_with_rows(Vec::new(), 0);
        }
        let mut names = Vec::new();
        let mut seen = HashSet::new();
        for record in records {
            for key in record.keys() {
                if seen.insert(key.as_str()) {
                    names.push(key.clone());
                }
            }
        }
        let mut columns = Vec::with_capacity(names.len());
        for name in &names {
            let mut values = Vec::with_capacity(records.len());
            for record in records {
                values.push(record.get(name).cloned().unwrap_or(Value::Null));
            }
            let data = ColumnArray::from_values(&values)
                .with_context(|| format!("Field '{name}' has mixed types"))?;
            columns.push(Column::new(name.clone(), data));
        }
        Self::try_with_rows(columns, records.len())
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn num_columns(&self) -> usize {
        self.columns.len()
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|column| column.name() == name)
    }

    pub fn to_records(&self) -> Vec<Record> {
        let mut records = Vec::with_capacity(self.num_rows);
        for row in 0..self.num_rows {
            let mut record = Record::new();
            for column in self.columns.iter() {
                record.insert(column.name(), column.data.value_at(row));
            }
            records.push(record);
        }
        records
    }

    /// Splits this batch into contiguous pieces of at most `batch_size` rows.
    pub fn split(&self, batch_size: usize) -> Vec<Self> {
        let batch_size = batch_size.max(1);
        if self.num_rows == 0 {
            return vec![self.clone()];
        }
        let mut batches = Vec::with_capacity(self.num_rows.div_ceil(batch_size));
        let mut start = 0;
        while start < self.num_rows {
            let end = (start + batch_size).min(self.num_rows);
            let indices: Vec<usize> = (start..end).collect();
            batches.push(self.take(&indices));
            start = end;
        }
        batches
    }

    pub(crate) fn empty_like(&self) -> Self {
        let columns: Vec<Column> = self
            .columns
            .iter()
            .map(|column| Column::new(column.name(), column.data.empty_like()))
            .collect();
        Self {
            columns: Arc::from(columns),
            num_rows: 0,
        }
    }

    pub(crate) fn take(&self, indices: &[usize]) -> Self {
        let columns: Vec<Column> = self
            .columns
            .iter()
            .map(|column| Column::new(column.name(), column.data.take(indices)))
            .collect();
        Self {
            columns: Arc::from(columns),
            num_rows: indices.len(),
        }
    }
}
