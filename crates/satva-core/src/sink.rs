use anyhow::Result;

use crate::record::Record;

pub trait Sink {
    fn write(&mut self, record: &Record) -> Result<()>;
}
