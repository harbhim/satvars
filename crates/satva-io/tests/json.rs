use std::fs;

use satva_core::source::Source;
use satva_io::source::JsonSource;

#[test]
fn test_json_source() {
    let path = std::env::temp_dir().join("people.json");

    fs::write(
        &path,
        r#"{"id":1,"name":"Alice","active":true}
{"id":2,"name":"Bob","active":false}
"#,
    )
    .unwrap();

    let source = JsonSource::new(&path);

    let rows: Vec<_> = source.read().unwrap().map(Result::unwrap).collect();

    assert_eq!(rows.len(), 2);

    assert_eq!(rows[0].require_string("name").unwrap(), "Alice");

    assert_eq!(rows[1].require_string("name").unwrap(), "Bob");
}

#[test]
fn pipeline_flushes_output_before_drop() {
    use satva_core::{Pipeline, PipelineOptions};
    use satva_io::sink::{CsvSink, JsonSink};
    let input =
        std::env::temp_dir().join(format!("satvars-flush-input-{}.json", std::process::id()));
    fs::write(&input, "{\"id\":1}\n").unwrap();
    for csv in [false, true] {
        let output = input.with_extension(if csv { "csv" } else { "jsonl" });
        let mut pipeline = Pipeline::new(Box::new(JsonSource::new(&input)));
        if csv {
            pipeline.set_sink(Box::new(CsvSink::new(&output)));
        } else {
            pipeline.set_sink(Box::new(JsonSink::new(&output)));
        }
        pipeline.run(PipelineOptions::default()).unwrap();
        assert_eq!(
            fs::read_to_string(&output).unwrap(),
            if csv { "id\n1\n" } else { "{\"id\":1}\n" }
        );
        drop(pipeline);
        fs::remove_file(output).unwrap();
    }
    fs::remove_file(input).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn json_flush_failure_is_reported() {
    use satva_core::Sink;
    let mut sink = satva_io::sink::JsonSink::new("/dev/full");
    sink.write(&satva_types::Record::new()).unwrap();
    assert!(sink.finish().is_err());
}
