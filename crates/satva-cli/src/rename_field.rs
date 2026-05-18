use anyhow::Result;

use satva_core::record::Record;
use satva_core::transformer::Transformer;

pub struct RenameField {
    from: String,
    to: String,
}

impl RenameField {
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
        }
    }
}

impl Transformer for RenameField {
    fn transform(&self, mut record: Record) -> Result<Record> {
        if let Some(value) = record.remove(&self.from) {
            record.insert(&self.to, value);
        }

        Ok(record)
    }
}
