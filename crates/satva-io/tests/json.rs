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
