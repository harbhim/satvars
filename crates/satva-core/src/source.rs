use anyhow::Result;

use crate::pipeline::Schema;
use crate::record::Record;

pub trait Source {
    fn read(&self) -> Result<Vec<Record>>;
    fn schema(&self) -> Result<Schema>;
}
