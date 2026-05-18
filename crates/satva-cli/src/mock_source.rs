use anyhow::Result;

use satva_core::record::Record;
use satva_core::source::Source;
use satva_core::value::Value;

pub struct MockSource;

impl Source for MockSource {
    fn read(&self) -> Result<Vec<Record>> {
        let mut record1 = Record::new();

        record1.insert("fname", Value::string("Hardik"));
        record1.insert("age", Value::int64(21));

        let mut record2 = Record::new();

        record2.insert("age", Value::int64(30));

        Ok(vec![record1, record2])
    }
}
