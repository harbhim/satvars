use crate::record::Record;
use anyhow::Result;

pub trait Source {
    fn read(&self) -> Result<Vec<Record>>;
}
