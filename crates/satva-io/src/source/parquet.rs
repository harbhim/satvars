use std::{fs::File, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use arrow::array::{Array, BooleanArray, Float64Array, Int64Array, StringArray, UInt64Array};
use arrow::compute::cast;
use arrow::datatypes::DataType;
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use satva_core::source::Source;
use satva_types::{Record, Value};

/// Reads a Parquet file into records.
///
/// Integers become `Int64`, floats become `Float64`, booleans stay booleans, and
/// strings stay strings. Dates, timestamps, and dictionary columns are read as strings.
/// `UInt64` values that do not fit in `i64` fail the row.
pub struct ParquetSource {
    path: PathBuf,
}

impl ParquetSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl Source for ParquetSource {
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>> {
        let file = File::open(&self.path)
            .with_context(|| format!("Failed to open {}", self.path.display()))?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)
            .with_context(|| format!("Failed to read Parquet {}", self.path.display()))?
            .build()
            .with_context(|| format!("Failed to read Parquet {}", self.path.display()))?;

        let iter = reader.flat_map(|batch| match batch {
            Ok(batch) => match records_from_batch(&batch) {
                Ok(records) => records.into_iter().map(Ok).collect(),
                Err(error) => vec![Err(error)],
            },
            Err(error) => vec![Err(anyhow!(error))],
        });

        Ok(Box::new(iter))
    }
}

enum Prepared {
    Null,
    Bool(BooleanArray),
    Int(Int64Array),
    Uint(UInt64Array),
    Float(Float64Array),
    Str(StringArray),
}

fn records_from_batch(batch: &RecordBatch) -> Result<Vec<Record>> {
    let columns = batch
        .columns()
        .iter()
        .enumerate()
        .map(|(index, column)| {
            let name = batch.schema().field(index).name().clone();
            let prepared = prepare_column(&name, column.as_ref())?;
            Ok((name, prepared))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut records = Vec::with_capacity(batch.num_rows());
    for row in 0..batch.num_rows() {
        let mut record = Record::new();
        for (name, column) in &columns {
            record.insert(name, column_value(column, row)?);
        }
        records.push(record);
    }
    Ok(records)
}

fn prepare_column(name: &str, array: &dyn Array) -> Result<Prepared> {
    match array.data_type() {
        DataType::Null => Ok(Prepared::Null),
        DataType::Boolean => Ok(Prepared::Bool(downcast(name, array)?)),
        DataType::Int64 => Ok(Prepared::Int(downcast(name, array)?)),
        DataType::UInt64 => Ok(Prepared::Uint(downcast(name, array)?)),
        DataType::Float64 => Ok(Prepared::Float(downcast(name, array)?)),
        DataType::Utf8 => Ok(Prepared::Str(downcast(name, array)?)),
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32 => Ok(Prepared::Int(cast_array(name, array, &DataType::Int64)?)),
        DataType::Float16 | DataType::Float32 => Ok(Prepared::Float(cast_array(
            name,
            array,
            &DataType::Float64,
        )?)),
        DataType::LargeUtf8
        | DataType::Timestamp(_, _)
        | DataType::Date32
        | DataType::Date64
        | DataType::Time32(_)
        | DataType::Time64(_)
        | DataType::Dictionary(_, _) => {
            Ok(Prepared::Str(cast_array(name, array, &DataType::Utf8)?))
        }
        other => Err(anyhow!(
            "Unsupported Parquet type {other} in column '{name}'"
        )),
    }
}

fn cast_array<T>(name: &str, array: &dyn Array, to_type: &DataType) -> Result<T>
where
    T: Array + Clone + 'static,
{
    let casted = cast(array, to_type)
        .with_context(|| format!("Failed to convert column '{name}' to {to_type}"))?;
    downcast(name, casted.as_ref())
}

fn downcast<T>(name: &str, array: &dyn Array) -> Result<T>
where
    T: Array + Clone + 'static,
{
    array
        .as_any()
        .downcast_ref::<T>()
        .cloned()
        .with_context(|| format!("Column '{name}' did not match its declared type"))
}

fn column_value(column: &Prepared, row: usize) -> Result<Value> {
    match column {
        Prepared::Null => Ok(Value::Null),
        Prepared::Bool(values) => {
            if values.is_null(row) {
                Ok(Value::Null)
            } else {
                Ok(Value::Boolean(values.value(row)))
            }
        }
        Prepared::Int(values) => {
            if values.is_null(row) {
                Ok(Value::Null)
            } else {
                Ok(Value::Int64(values.value(row)))
            }
        }
        Prepared::Float(values) => {
            if values.is_null(row) {
                Ok(Value::Null)
            } else {
                Ok(Value::Float64(values.value(row)))
            }
        }
        Prepared::Str(values) => {
            if values.is_null(row) {
                Ok(Value::Null)
            } else {
                Ok(Value::String(values.value(row).to_string()))
            }
        }
        Prepared::Uint(values) => {
            if values.is_null(row) {
                Ok(Value::Null)
            } else {
                let value = values.value(row);
                i64::try_from(value)
                    .map(Value::Int64)
                    .with_context(|| format!("uint64 value {value} does not fit in int64"))
            }
        }
    }
}
