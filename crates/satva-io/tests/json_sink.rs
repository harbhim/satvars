use std::fs;

use satva_core::{record::Record, sink::Sink, value::Value};
use satva_io::sink::JsonSink;

#[test]
fn writes_json_lines() {
    let path = std::env::temp_dir().join("satva-json-sink.json");

    let mut sink = JsonSink::new(&path);

    let mut record = Record::new();
    record.insert("id", Value::Int64(1));
    record.insert("name", Value::String("Alice".into()));
    record.insert("active", Value::Boolean(true));

    sink.write(&record).unwrap();

    drop(sink);

    let contents = fs::read_to_string(path).unwrap();

    assert_eq!(contents.trim(), r#"{"active":true,"id":1,"name":"Alice"}"#);
}
