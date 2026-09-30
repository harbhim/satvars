use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use satva_core::sink::Sink;
use satva_core::source::Source;
use satva_io::{
    ExcelSink, ExcelSource, JsonArraySink, JsonArraySource, ParquetSink, ParquetSource, TsvSink,
    TsvSource,
};
use satva_types::{Record, Value};

static NEXT_FILE_ID: AtomicUsize = AtomicUsize::new(0);

fn temp_path(name: &str, extension: &str) -> PathBuf {
    let unique_id = NEXT_FILE_ID.fetch_add(1, Ordering::SeqCst);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "satva_io_{name}_{timestamp}_{unique_id}.{extension}"
    ))
}

fn record(fields: &[(&str, Value)]) -> Record {
    let mut record = Record::new();
    for (key, value) in fields {
        record.insert(key, value.clone());
    }
    record
}

#[test]
fn tsv_round_trip() {
    let path = temp_path("tsv", "tsv");
    let mut sink = TsvSink::new(&path);
    sink.write(&record(&[
        ("name", Value::string("Asha")),
        ("city", Value::string("Pune")),
    ]))
    .unwrap();
    sink.write(&record(&[
        ("name", Value::string("Ravi")),
        ("city", Value::string("Mumbai")),
    ]))
    .unwrap();
    sink.finish().unwrap();

    let records: Vec<_> = TsvSource::new(&path)
        .read()
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records[0].get("name"), Some(&Value::string("Asha")));
    assert_eq!(records[1].get("city"), Some(&Value::string("Mumbai")));
    fs::remove_file(path).unwrap();
}

#[test]
fn json_array_round_trip() {
    let path = temp_path("json_array", "json");
    let mut sink = JsonArraySink::new(&path);
    sink.write(&record(&[
        ("id", Value::Int64(1)),
        ("name", Value::string("Ada")),
        ("active", Value::Boolean(true)),
    ]))
    .unwrap();
    sink.write(&record(&[
        ("id", Value::Int64(2)),
        ("name", Value::string("Ben")),
        ("active", Value::Boolean(false)),
    ]))
    .unwrap();
    sink.finish().unwrap();

    let records: Vec<_> = JsonArraySource::new(&path)
        .read()
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].get("name"), Some(&Value::string("Ada")));
    assert_eq!(records[1].get("id"), Some(&Value::Int64(2)));
    assert_eq!(records[1].get("active"), Some(&Value::Boolean(false)));
    fs::remove_file(path).unwrap();
}

#[test]
fn json_array_finish_writes_empty_array() {
    let path = temp_path("json_array_empty", "json");
    let mut sink = JsonArraySink::new(&path);
    sink.finish().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "[]\n");
    fs::remove_file(path).unwrap();
}

#[test]
fn parquet_round_trip_keeps_scalar_types() {
    let path = temp_path("parquet", "parquet");
    let mut sink = ParquetSink::new(&path);
    sink.write(&record(&[
        ("id", Value::Int64(1)),
        ("score", Value::Float64(1.5)),
        ("active", Value::Boolean(true)),
        ("name", Value::string("Ada")),
    ]))
    .unwrap();
    sink.write(&record(&[
        ("id", Value::Int64(2)),
        ("score", Value::Null),
        ("active", Value::Boolean(false)),
        ("name", Value::string("Ben")),
    ]))
    .unwrap();
    sink.finish().unwrap();

    let records: Vec<_> = ParquetSource::new(&path)
        .read()
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records[0].get("id"), Some(&Value::Int64(1)));
    assert_eq!(records[0].get("score"), Some(&Value::Float64(1.5)));
    assert_eq!(records[0].get("active"), Some(&Value::Boolean(true)));
    assert_eq!(records[0].get("name"), Some(&Value::string("Ada")));
    assert_eq!(records[1].get("score"), Some(&Value::Null));
    assert_eq!(records[1].get("name"), Some(&Value::string("Ben")));
    fs::remove_file(path).unwrap();
}

#[test]
fn parquet_widens_integer_column_to_float() {
    let path = temp_path("parquet_widen", "parquet");
    let mut sink = ParquetSink::new(&path);
    sink.write(&record(&[("n", Value::Int64(1))])).unwrap();
    sink.write(&record(&[("n", Value::Float64(2.5))])).unwrap();
    sink.finish().unwrap();

    let records: Vec<_> = ParquetSource::new(&path)
        .read()
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records[0].get("n"), Some(&Value::Float64(1.0)));
    assert_eq!(records[1].get("n"), Some(&Value::Float64(2.5)));
    fs::remove_file(path).unwrap();
}

#[test]
fn excel_round_trip_reads_typed_cells() {
    let path = temp_path("excel", "xlsx");
    let mut sink = ExcelSink::with_sheet(&path, "People");
    sink.write(&record(&[
        ("name", Value::string("Asha")),
        ("age", Value::Int64(31)),
        ("active", Value::Boolean(true)),
    ]))
    .unwrap();
    sink.write(&record(&[
        ("name", Value::string("Ravi")),
        ("age", Value::Int64(28)),
        ("active", Value::Boolean(false)),
    ]))
    .unwrap();
    sink.finish().unwrap();

    let records: Vec<_> = ExcelSource::with_sheet(&path, "People")
        .read()
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records[0].get("name"), Some(&Value::string("Asha")));
    assert_eq!(records[0].get("age"), Some(&Value::Int64(31)));
    assert_eq!(records[0].get("active"), Some(&Value::Boolean(true)));
    assert_eq!(records[1].get("name"), Some(&Value::string("Ravi")));
    fs::remove_file(path).unwrap();
}
