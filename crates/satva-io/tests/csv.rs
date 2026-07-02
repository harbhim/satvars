use anyhow::Result;
use satva_core::record::Record;
use satva_core::sink::Sink;
use satva_core::source::Source;
use satva_core::value::Value;
use satva_io::{CsvSink, CsvSource};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_FILE_ID: AtomicUsize = AtomicUsize::new(0);

fn temp_csv_path(name: &str) -> PathBuf {
    let unique_id = NEXT_FILE_ID.fetch_add(1, Ordering::SeqCst);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!(
        "satva_io_{name}_{timestamp}_{unique_id}.csv"
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
fn csv_source_reads_records_with_header_fields() -> Result<()> {
    let path = temp_csv_path("source_reads_records");
    fs::write(&path, "name,age,city\nAsha,31,Pune\nRavi,28,Mumbai\n")?;

    let source = CsvSource::new(&path);
    let records: Vec<Record> = source.read()?.collect::<Result<Vec<_>, _>>()?;

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].get("name"), Some(&Value::string("Asha")));
    assert_eq!(records[0].get("age"), Some(&Value::string("31")));
    assert_eq!(records[0].get("city"), Some(&Value::string("Pune")));
    assert_eq!(records[1].get("name"), Some(&Value::string("Ravi")));
    assert_eq!(records[1].get("age"), Some(&Value::string("28")));
    assert_eq!(records[1].get("city"), Some(&Value::string("Mumbai")));

    fs::remove_file(path)?;

    Ok(())
}

#[test]
fn csv_sink_writes_headers_and_rows() -> Result<()> {
    let path = temp_csv_path("sink_writes_rows");
    let mut sink = CsvSink::new(&path);

    sink.write(&record(&[
        ("name", Value::string("Asha")),
        ("age", Value::Int64(31)),
    ]))?;
    sink.write(&record(&[
        ("name", Value::string("Ravi")),
        ("age", Value::Int64(28)),
    ]))?;
    drop(sink);

    let output = fs::read_to_string(&path)?;

    assert_eq!(output, "age,name\n31,Asha\n28,Ravi\n");

    fs::remove_file(path)?;

    Ok(())
}

#[test]
fn csv_sink_writes_missing_fields_as_empty_cells() -> Result<()> {
    let path = temp_csv_path("sink_missing_fields");
    let mut sink = CsvSink::new(&path);

    sink.write(&record(&[
        ("name", Value::string("Asha")),
        ("age", Value::Int64(31)),
    ]))?;
    sink.write(&record(&[("name", Value::string("Ravi"))]))?;
    drop(sink);

    let output = fs::read_to_string(&path)?;

    assert_eq!(output, "age,name\n31,Asha\n,Ravi\n");

    fs::remove_file(path)?;

    Ok(())
}
