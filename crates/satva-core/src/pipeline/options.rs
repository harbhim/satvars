#[derive(Debug, Clone, Copy)]
pub struct PipelineOptions {
    collect_logs: bool,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self { collect_logs: true }
    }
}

impl PipelineOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_logs() -> Self {
        Self { collect_logs: true }
    }

    pub fn without_logs() -> Self {
        Self {
            collect_logs: false,
        }
    }

    pub fn collect_logs(&self) -> bool {
        self.collect_logs
    }
}
