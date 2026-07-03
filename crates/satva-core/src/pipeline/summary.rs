#[derive(Debug, Default)]
pub struct PipelineSummary {
    pub processed: usize,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl PipelineSummary {
    pub fn record_processed(&mut self) {
        self.processed += 1;
    }

    pub fn record_succeeded(&mut self) {
        self.succeeded += 1;
    }

    pub fn record_skipped(&mut self) {
        self.skipped += 1;
    }

    pub fn record_failed(&mut self) {
        self.failed += 1;
    }
}
