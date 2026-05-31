#[derive(Debug, Default)]
pub struct PipelineSummary {
    pub processed: usize,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
}
