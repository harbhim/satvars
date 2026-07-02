use anyhow::Result;

use crate::record::Record;

pub trait Source {
    fn read(&self) -> Result<Box<dyn Iterator<Item = Result<Record>>>>;

    fn read_sample(&self, limit: usize) -> Result<Vec<Record>> {
        let iter = self.read()?;
        let mut sample = Vec::new();
        for item in iter.take(limit) {
            sample.push(item?);
        }
        Ok(sample)
    }
}
