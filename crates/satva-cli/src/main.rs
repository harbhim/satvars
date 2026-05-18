use satva_core::record::Record;
use satva_core::value::Value;

fn main() {
    let mut record = Record::new();

    record.insert("name", Value::string("Hardik"));
    record.insert("age", Value::int64(21));

    if let Some(value) = record.get("name") {
        if let Some(name) = value.as_string() {
            println!("Name: {}", name);
        }
    }

    if let Some(value) = record.get("age") {
        if let Some(age) = value.as_i64() {
            println!("Age: {}", age);
        }
    }
}
