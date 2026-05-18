use anyhow::Result;

use crate::record::Record;

pub trait Transformer {
    fn transform(&self, record: Record) -> Result<Record>;
}
