use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

use satva_core::{
    Pipeline, PipelineStage, RemoveFieldStage, RenameFieldStage, SchemaValidation,
    SelectFieldsStage, Sink, Source,
};
use satva_io::sink::{CsvSink, JsonSink};
use satva_io::source::{CsvSource, JsonSource};
use satva_types::Schema;

#[derive(Debug, Deserialize)]
pub struct PipelineConfig {
    pub source: SourceConfig,
    pub sink: Option<SinkConfig>,
    #[serde(default)]
    pub schema: SchemaConfig,
    #[serde(default)]
    pub stages: Vec<StageConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceConfig {
    Json { path: PathBuf },
    Csv { path: PathBuf },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SinkConfig {
    Json { path: PathBuf },
    Csv { path: PathBuf },
}

#[derive(Debug, Deserialize, Default)]
pub struct SchemaConfig {
    #[serde(default)]
    pub infer: bool,
    #[serde(default = "default_sample_size")]
    pub sample_size: usize,
}

fn default_sample_size() -> usize {
    1000
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StageConfig {
    RenameField {
        from: String,
        to: String,
    },
    SelectFields {
        fields: Vec<String>,
    },
    RemoveField {
        fields: Vec<String>,
    },
    SchemaValidation,
    /// Recognized but not runnable yet: needs satva-parser to turn the
    /// string into an `Expression`. Kept here (rather than left out of the
    /// enum) so config files can already describe the intent and get a
    /// clear, actionable error instead of a generic "unknown variant".
    /// Fields are unread until that milestone wires them into `build_stage`.
    #[allow(dead_code)]
    Filter {
        expression: String,
    },
    #[allow(dead_code)]
    SetField {
        field: String,
        expression: String,
    },
}

impl StageConfig {
    fn type_name(&self) -> &'static str {
        match self {
            StageConfig::RenameField { .. } => "rename_field",
            StageConfig::SelectFields { .. } => "select_fields",
            StageConfig::RemoveField { .. } => "remove_field",
            StageConfig::SchemaValidation => "schema_validation",
            StageConfig::Filter { .. } => "filter",
            StageConfig::SetField { .. } => "set_field",
        }
    }
}

impl PipelineConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        serde_yaml::from_str(&text)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))
    }

    /// Builds a runnable `Pipeline` from this config, plus the inferred
    /// schema (if `schema.infer` was requested) for the caller to print.
    pub fn build(self) -> Result<(Pipeline, Option<Schema>)> {
        let source = build_source(&self.source);

        let schema = if self.schema.infer {
            let sample = source
                .read_sample(self.schema.sample_size)
                .context("Failed to read sample for schema inference")?;
            Some(Schema::infer(&sample))
        } else {
            None
        };

        let mut pipeline = Pipeline::new(source);

        for stage_config in &self.stages {
            pipeline.add_stage(build_stage(stage_config, schema.as_ref())?);
        }

        if let Some(sink_config) = &self.sink {
            pipeline.set_sink(build_sink(sink_config));
        }

        Ok((pipeline, schema))
    }
}

fn build_source(config: &SourceConfig) -> Box<dyn Source> {
    match config {
        SourceConfig::Json { path } => Box::new(JsonSource::new(path.clone())),
        SourceConfig::Csv { path } => Box::new(CsvSource::new(path.clone())),
    }
}

fn build_sink(config: &SinkConfig) -> Box<dyn Sink> {
    match config {
        SinkConfig::Json { path } => Box::new(JsonSink::new(path.clone())),
        SinkConfig::Csv { path } => Box::new(CsvSink::new(path.clone())),
    }
}

fn build_stage(config: &StageConfig, schema: Option<&Schema>) -> Result<Box<dyn PipelineStage>> {
    let stage: Box<dyn PipelineStage> = match config {
        StageConfig::RenameField { from, to } => {
            Box::new(RenameFieldStage::new(from.clone(), to.clone()))
        }

        StageConfig::SelectFields { fields } => Box::new(SelectFieldsStage::new(fields.clone())),

        StageConfig::RemoveField { fields } => Box::new(RemoveFieldStage::new(fields.clone())),

        StageConfig::SchemaValidation => {
            let schema = schema.cloned().ok_or_else(|| {
                anyhow!(
                    "stage 'schema_validation' requires 'schema: {{ infer: true }}' in the config"
                )
            })?;
            Box::new(SchemaValidation::new(schema))
        }

        StageConfig::Filter { .. } | StageConfig::SetField { .. } => {
            bail!(
                "stage '{}' needs the expression parser (satva-parser), which isn't wired up \
                 yet — build it in Rust directly for now via PipelineBuilder",
                config.type_name()
            )
        }
    };

    Ok(stage)
}
