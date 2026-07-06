pub mod log;
pub mod options;
// The sub-module `pipeline` is named the same as the parent module `pipeline` to contain the main pipeline struct definition.
#[expect(clippy::module_inception)]
pub mod pipeline;
pub mod run_result;
pub mod runtime;
pub mod summary;

pub use log::PipelineLog;
pub use options::PipelineOptions;
pub use pipeline::Pipeline;
pub use run_result::PipelineRunResult;
pub use runtime::context::StageContext;
pub use runtime::error::StageError;
pub use runtime::executor::StageExecutor;
pub use runtime::result::StageResult;
pub use runtime::stage::PipelineStage;
pub use summary::PipelineSummary;
