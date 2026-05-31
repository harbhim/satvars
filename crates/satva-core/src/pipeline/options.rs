#[derive(Debug, Clone, Copy)]
pub struct PipelineOptions {
    pub collect_logs: bool,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self { collect_logs: true }
    }
}
