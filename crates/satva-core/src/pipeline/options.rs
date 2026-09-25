/// Policy for stage and sink-write failures. Source and finish errors always fail the run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ErrorPolicy {
    /// Keep processing; callers must inspect `summary.failed`.
    #[default]
    Continue,
    /// Return the first record failure after finishing the sink.
    StopOnError,
}

#[derive(Debug, Clone, Copy)]
pub struct PipelineOptions {
    pub collect_logs: bool,
    pub error_policy: ErrorPolicy,
    /// Maximum retained logs; `None` explicitly enables unlimited retention.
    pub log_limit: Option<usize>,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            collect_logs: true,
            error_policy: ErrorPolicy::Continue,
            log_limit: Some(1_000),
        }
    }
}

impl PipelineOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_logs() -> Self {
        Self::default()
    }
    pub fn without_logs() -> Self {
        Self {
            collect_logs: false,
            ..Self::default()
        }
    }
    #[must_use]
    pub fn with_error_policy(mut self, policy: ErrorPolicy) -> Self {
        self.error_policy = policy;
        self
    }
    #[must_use]
    pub fn with_log_limit(mut self, limit: Option<usize>) -> Self {
        self.log_limit = limit;
        self
    }
    pub fn collect_logs(&self) -> bool {
        self.collect_logs
    }
    pub(crate) fn should_log(&self, count: usize) -> bool {
        self.collect_logs && self.log_limit.is_none_or(|limit| count < limit)
    }
}
