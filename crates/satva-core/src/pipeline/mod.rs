pub mod log;
pub mod options;
// The sub-module `pipeline` is named the same as the parent module `pipeline` to contain the main pipeline struct definition.
#[expect(clippy::module_inception)]
pub mod pipeline;
pub mod run_result;
pub mod stage;
pub mod summary;

pub use log::PipelineLog;
pub use options::PipelineOptions;
pub use pipeline::Pipeline;
pub use run_result::PipelineRunResult;
pub use stage::context::StageContext;
pub use stage::error::StageError;
pub use stage::pipeline_stage::PipelineStage;
pub use stage::result::StageResult;
pub use stage::schema_validation::SchemaValidation;
pub use summary::PipelineSummary;
