use std::{fs::File, path::PathBuf, sync::Arc};

use anyhow::{Context, Result, anyhow};
use arrow::array::{
    ArrayRef, BooleanBuilder, Float64Builder, Int64Builder, RecordBatch, StringBuilder,
};
use arrow::datatypes::{DataType, Field, Schema};
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use satva_core::sink::Sink;
use satva_types::{Record, Value};

/// Writes a Parquet file. Rows stay in memory until [`Sink::finish`].
///
/// Columns and their types come from the first record. A later float widens an
/// integer column. Missing fields are null. Extra fields are dropped.
pub struct ParquetSink {
    path: PathBuf,
    columns: Vec<ColumnBuf>,
    started: bool,
}

struct ColumnBuf {
    name: String,
    data: ColData,
}

enum ColData {
    Nulls(usize),
    Int(Vec<Option<i64>>),
    Float(Vec<Option<f64>>),
    Bool(Vec<Option<bool>>),
    Str(Vec<Option<String>>),
}

impl ParquetSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            columns: Vec::new(),
            started: false,
        }
    }
}

impl Sink for ParquetSink {
    fn finish(&mut self) -> Result<()> {
        if !self.started {
            return Ok(());
        }

        let mut fields = Vec::with_capacity(self.columns.len());
        let mut arrays = Vec::with_capacity(self.columns.len());
        for column in &self.columns {
            let (field, array) = column_array(column);
            fields.push(field);
            arrays.push(array);
        }
        let schema = Arc::new(Schema::new(fields));
        let batch = RecordBatch::try_new(Arc::clone(&schema), arrays)
            .context("Failed to build Parquet batch")?;

        let file = File::create(&self.path)
            .with_context(|| format!("Failed to create {}", self.path.display()))?;
        let properties = WriterProperties::builder()
            .set_compression(Compression::UNCOMPRESSED)
            .build();
        let mut writer = parquet::arrow::ArrowWriter::try_new(file, schema, Some(properties))
            .context("Failed to create Parquet writer")?;
        writer
            .write(&batch)
            .context("Failed to write Parquet batch")?;
        writer.close().context("Failed to close Parquet file")?;

        self.columns.clear();
        self.started = false;
        Ok(())
    }

    fn write(&mut self, record: &Record) -> Result<()> {
        if !self.started {
            if record.is_empty() {
                return Err(anyhow!(
                    "Parquet sink cannot infer columns from an empty record"
                ));
            }
            self.columns = record
                .keys()
                .map(|name| ColumnBuf {
                    name: name.clone(),
                    data: ColData::Nulls(0),
                })
                .collect();
            self.started = true;
        }

        for column in &self.columns {
            let value = record.get(&column.name).unwrap_or(&Value::Null);
            if !compatible(&column.data, value) {
                return Err(anyhow!(
                    "Column '{}' is {} and cannot store {}",
                    column.name,
                    type_name(&column.data),
                    value_kind(value)
                ));
            }
        }

        for column in &mut self.columns {
            let value = record.get(&column.name).unwrap_or(&Value::Null);
            append_cell(&mut column.data, value);
        }
        Ok(())
    }
}

fn compatible(data: &ColData, value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Int64(_) => matches!(
            data,
            ColData::Nulls(_) | ColData::Int(_) | ColData::Float(_)
        ),
        Value::Float64(_) => {
            matches!(
                data,
                ColData::Nulls(_) | ColData::Int(_) | ColData::Float(_)
            )
        }
        Value::Boolean(_) => matches!(data, ColData::Nulls(_) | ColData::Bool(_)),
        Value::String(_) => matches!(data, ColData::Nulls(_) | ColData::Str(_)),
    }
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Int64(_) => "an integer",
        Value::Float64(_) => "a float",
        Value::Boolean(_) => "a boolean",
        Value::String(_) => "a string",
    }
}

fn append_cell(data: &mut ColData, value: &Value) {
    match value {
        Value::Null => match data {
            ColData::Nulls(count) => *count += 1,
            ColData::Int(values) => values.push(None),
            ColData::Float(values) => values.push(None),
            ColData::Bool(values) => values.push(None),
            ColData::Str(values) => values.push(None),
        },
        Value::Int64(value) => {
            *data = match std::mem::replace(data, ColData::Nulls(0)) {
                ColData::Nulls(count) => {
                    let mut values = vec![None; count];
                    values.push(Some(*value));
                    ColData::Int(values)
                }
                ColData::Int(mut values) => {
                    values.push(Some(*value));
                    ColData::Int(values)
                }
                ColData::Float(mut values) => {
                    values.push(Some(*value as f64));
                    ColData::Float(values)
                }
                other => other,
            };
        }
        Value::Float64(value) => {
            *data = match std::mem::replace(data, ColData::Nulls(0)) {
                ColData::Nulls(count) => {
                    let mut values = vec![None; count];
                    values.push(Some(*value));
                    ColData::Float(values)
                }
                ColData::Int(values) => {
                    let mut widened: Vec<Option<f64>> = values
                        .into_iter()
                        .map(|item| item.map(|item| item as f64))
                        .collect();
                    widened.push(Some(*value));
                    ColData::Float(widened)
                }
                ColData::Float(mut values) => {
                    values.push(Some(*value));
                    ColData::Float(values)
                }
                other => other,
            };
        }
        Value::Boolean(value) => {
            *data = match std::mem::replace(data, ColData::Nulls(0)) {
                ColData::Nulls(count) => {
                    let mut values = vec![None; count];
                    values.push(Some(*value));
                    ColData::Bool(values)
                }
                ColData::Bool(mut values) => {
                    values.push(Some(*value));
                    ColData::Bool(values)
                }
                other => other,
            };
        }
        Value::String(value) => {
            *data = match std::mem::replace(data, ColData::Nulls(0)) {
                ColData::Nulls(count) => {
                    let mut values = vec![None; count];
                    values.push(Some(value.clone()));
                    ColData::Str(values)
                }
                ColData::Str(mut values) => {
                    values.push(Some(value.clone()));
                    ColData::Str(values)
                }
                other => other,
            };
        }
    }
}

fn type_name(data: &ColData) -> &'static str {
    match data {
        ColData::Nulls(_) => "null",
        ColData::Int(_) => "an integer",
        ColData::Float(_) => "a float",
        ColData::Bool(_) => "a boolean",
        ColData::Str(_) => "a string",
    }
}

fn column_array(column: &ColumnBuf) -> (Field, ArrayRef) {
    let (data_type, array) = match &column.data {
        ColData::Nulls(count) => {
            let mut builder = Int64Builder::with_capacity(*count);
            for _ in 0..*count {
                builder.append_null();
            }
            (DataType::Int64, Arc::new(builder.finish()) as ArrayRef)
        }
        ColData::Int(values) => {
            let mut builder = Int64Builder::with_capacity(values.len());
            for value in values {
                match value {
                    Some(value) => builder.append_value(*value),
                    None => builder.append_null(),
                }
            }
            (DataType::Int64, Arc::new(builder.finish()) as ArrayRef)
        }
        ColData::Float(values) => {
            let mut builder = Float64Builder::with_capacity(values.len());
            for value in values {
                match value {
                    Some(value) => builder.append_value(*value),
                    None => builder.append_null(),
                }
            }
            (DataType::Float64, Arc::new(builder.finish()) as ArrayRef)
        }
        ColData::Bool(values) => {
            let mut builder = BooleanBuilder::with_capacity(values.len());
            for value in values {
                match value {
                    Some(value) => builder.append_value(*value),
                    None => builder.append_null(),
                }
            }
            (DataType::Boolean, Arc::new(builder.finish()) as ArrayRef)
        }
        ColData::Str(values) => {
            let mut builder = StringBuilder::with_capacity(values.len(), values.len() * 8);
            for value in values {
                match value {
                    Some(value) => builder.append_value(value),
                    None => builder.append_null(),
                }
            }
            (DataType::Utf8, Arc::new(builder.finish()) as ArrayRef)
        }
    };

    (Field::new(&column.name, data_type, true), array)
}
