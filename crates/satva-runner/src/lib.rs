mod config;

pub use config::{PipelineConfig, SchemaConfig, SinkConfig, SourceConfig, StageConfig};

use std::path::Path;

use anyhow::Result;
use satva_core::{PipelineLog, PipelineOptions, PipelineSummary};
use satva_types::Schema;

/// Outcome of a YAML pipeline run.
///
/// File paths inside the config are resolved from the process working directory.
#[derive(Debug)]
pub struct PipelineRunReport {
    pub schema: Option<Schema>,
    pub summary: PipelineSummary,
    pub logs: Vec<PipelineLog>,
}

/// Load a YAML pipeline config, run it, and return the schema, summary, and logs.
pub fn run_yaml(path: impl AsRef<Path>, options: PipelineOptions) -> Result<PipelineRunReport> {
    let config = PipelineConfig::load(path.as_ref())?;
    run_config(config, options)
}

/// Run an already loaded pipeline config.
pub fn run_config(config: PipelineConfig, options: PipelineOptions) -> Result<PipelineRunReport> {
    let (mut pipeline, schema) = config.build()?;
    let result = pipeline.run(options)?;
    Ok(PipelineRunReport {
        schema,
        summary: result.summary,
        logs: result.logs,
    })
}
