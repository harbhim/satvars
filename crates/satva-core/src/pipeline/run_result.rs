use super::log::PipelineLog;
use super::summary::PipelineSummary;

#[derive(Debug)]
pub struct PipelineRunResult {
    pub summary: PipelineSummary,
    pub logs: Vec<PipelineLog>,
}
