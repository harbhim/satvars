use anyhow::Result;

use satva_types::Record;

pub trait Sink {
    fn write(&mut self, record: &Record) -> Result<()>;
}
