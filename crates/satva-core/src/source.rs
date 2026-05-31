use anyhow::Result;

use crate::record::Record;

pub trait Source {
    fn read(&self) -> Result<Vec<Record>>;
}
