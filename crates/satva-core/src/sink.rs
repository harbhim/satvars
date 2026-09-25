use anyhow::Result;

use satva_types::Record;

pub trait Sink {
    /// Flush pending output before a run returns, including after a processing error.
    /// This is not a transaction commit or a guarantee of disk durability.
    fn finish(&mut self) -> Result<()> {
        Ok(())
    }

    fn write(&mut self, record: &Record) -> Result<()>;
}
