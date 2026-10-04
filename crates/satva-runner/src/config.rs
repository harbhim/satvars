use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde::de::{self, Deserializer, SeqAccess, Visitor};

use satva_core::{
    ExternalCompare, ExternalData, ExternalPolicy, ExternalStage, ExternalTable, ExternalValues,
    FilterStage, Pipeline, PipelineStage, RemoveFieldStage, RenameFieldStage, SchemaValidation,
    SelectFieldsStage, SetFieldStage, Sink, Source,
};
use satva_io::sink::{CsvSink, ExcelSink, JsonArraySink, JsonSink, ParquetSink, TsvSink};
use satva_io::source::{
    CsvSource, ExcelSource, JsonArraySource, JsonSource, ParquetSource, TsvSource,
};
use satva_types::{Record, Schema};

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
    Json {
        path: PathBuf,
    },
    JsonArray {
        path: PathBuf,
    },
    Csv {
        path: PathBuf,
    },
    Tsv {
        path: PathBuf,
    },
    Parquet {
        path: PathBuf,
    },
    Excel {
        path: PathBuf,
        #[serde(default)]
        sheet: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SinkConfig {
    Json {
        path: PathBuf,
    },
    JsonArray {
        path: PathBuf,
    },
    Csv {
        path: PathBuf,
    },
    Tsv {
        path: PathBuf,
    },
    Parquet {
        path: PathBuf,
    },
    Excel {
        path: PathBuf,
        #[serde(default)]
        sheet: Option<String>,
    },
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
    Filter {
        expression: String,
    },
    SetField {
        field: String,
        expression: String,
    },
    /// Compares each row with external values, then continues, skips, fails, or replaces fields.
    External {
        #[serde(deserialize_with = "string_or_list")]
        key: Vec<String>,
        #[serde(default)]
        compare: Option<Vec<String>>,
        #[serde(default)]
        on_missing: Option<PolicyConfig>,
        #[serde(default)]
        on_match: Option<PolicyConfig>,
        #[serde(default)]
        on_differ: Option<PolicyConfig>,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        path: Option<PathBuf>,
        #[serde(default)]
        format: Option<String>,
        #[serde(default)]
        sheet: Option<String>,
    },
}

/// What an `external` stage does when values are missing, equal, or different.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyConfig {
    Continue,
    Skip,
    Fail,
    #[serde(alias = "correct")]
    Replace,
}

impl SourceConfig {
    /// Source type name used in YAML (`json`, `csv`, `excel`, and so on).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Json { .. } => "json",
            Self::JsonArray { .. } => "json_array",
            Self::Csv { .. } => "csv",
            Self::Tsv { .. } => "tsv",
            Self::Parquet { .. } => "parquet",
            Self::Excel { .. } => "excel",
        }
    }

    /// Replace the source path. The source type stays the same.
    pub fn set_path(&mut self, new_path: PathBuf) {
        match self {
            Self::Json { path }
            | Self::JsonArray { path }
            | Self::Csv { path }
            | Self::Tsv { path }
            | Self::Parquet { path } => *path = new_path,
            Self::Excel { path, .. } => *path = new_path,
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
        self.build_with(&HashMap::new())
    }

    /// Same as [`Self::build`], with values for named `external` stages.
    ///
    /// A stage with `path` reads that file and does not use `externals`.
    /// A stage with `name` requires a matching entry.
    pub fn build_with(
        self,
        externals: &HashMap<String, ExternalData>,
    ) -> Result<(Pipeline, Option<Schema>)> {
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
            pipeline.add_stage(build_stage(stage_config, schema.as_ref(), externals)?);
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
        SourceConfig::JsonArray { path } => Box::new(JsonArraySource::new(path.clone())),
        SourceConfig::Csv { path } => Box::new(CsvSource::new(path.clone())),
        SourceConfig::Tsv { path } => Box::new(TsvSource::new(path.clone())),
        SourceConfig::Parquet { path } => Box::new(ParquetSource::new(path.clone())),
        SourceConfig::Excel { path, sheet } => Box::new(match sheet {
            Some(sheet) => ExcelSource::with_sheet(path.clone(), sheet.clone()),
            None => ExcelSource::new(path.clone()),
        }),
    }
}

fn build_sink(config: &SinkConfig) -> Box<dyn Sink> {
    match config {
        SinkConfig::Json { path } => Box::new(JsonSink::new(path.clone())),
        SinkConfig::JsonArray { path } => Box::new(JsonArraySink::new(path.clone())),
        SinkConfig::Csv { path } => Box::new(CsvSink::new(path.clone())),
        SinkConfig::Tsv { path } => Box::new(TsvSink::new(path.clone())),
        SinkConfig::Parquet { path } => Box::new(ParquetSink::new(path.clone())),
        SinkConfig::Excel { path, sheet } => Box::new(match sheet {
            Some(sheet) => ExcelSink::with_sheet(path.clone(), sheet.clone()),
            None => ExcelSink::new(path.clone()),
        }),
    }
}

fn build_stage(
    config: &StageConfig,
    schema: Option<&Schema>,
    externals: &HashMap<String, ExternalData>,
) -> Result<Box<dyn PipelineStage>> {
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

        StageConfig::Filter { expression } => {
            let expr = satva_parser::parse_expression(expression)
                .map_err(|e| anyhow!("Failed to parse filter expression: {e}"))?;
            Box::new(FilterStage::new(expr))
        }

        StageConfig::SetField { field, expression } => {
            let expr = satva_parser::parse_expression(expression)
                .map_err(|e| anyhow!("Failed to parse set_field expression: {e}"))?;
            Box::new(SetFieldStage::new(field.clone(), expr))
        }

        StageConfig::External {
            key,
            compare,
            on_missing,
            on_match,
            on_differ,
            name,
            path,
            format,
            sheet,
        } => Box::new(build_external(
            key,
            compare,
            *on_missing,
            *on_match,
            *on_differ,
            name,
            path,
            format,
            sheet,
            externals,
        )?),
    };

    Ok(stage)
}

fn build_external(
    key: &[String],
    compare: &Option<Vec<String>>,
    on_missing: Option<PolicyConfig>,
    on_match: Option<PolicyConfig>,
    on_differ: Option<PolicyConfig>,
    name: &Option<String>,
    path: &Option<PathBuf>,
    format: &Option<String>,
    sheet: &Option<String>,
    externals: &HashMap<String, ExternalData>,
) -> Result<ExternalStage> {
    let compare = ExternalCompare {
        key: key.to_vec(),
        compare: (*compare).clone(),
        on_missing: policy(on_missing, ExternalPolicy::Continue),
        on_match: policy(on_match, ExternalPolicy::Skip),
        on_differ: policy(on_differ, ExternalPolicy::Replace),
    };

    let (label, values) = match (name, path) {
        (Some(name), Some(path)) => {
            return Err(anyhow!(
                "external stage '{name}' has both name and path ({}). Use a file path or supplied values, not both",
                path.display()
            ));
        }
        (None, None) => {
            return Err(anyhow!(
                "external stage needs a name for supplied values or a path to a file of values"
            ));
        }
        (Some(name), None) => {
            if name.is_empty() {
                return Err(anyhow!("external stage name must not be empty"));
            }
            let data = externals.get(name).ok_or_else(|| {
                anyhow!("external stage '{name}' has no values. Pass them in externals, or set path to a file")
            })?;
            (name.clone(), values_from_data(name, key, data)?)
        }
        (None, Some(path)) => {
            let format = match format {
                Some(format) => format.clone(),
                None => infer_format(path)?,
            };
            let records = read_external_file(path, &format, sheet.as_deref())?;
            let table = ExternalTable::from_records(key, records)
                .map_err(|err| anyhow!("{path}: {err}", path = path.display()))?;
            (
                path.display().to_string(),
                Arc::new(table) as Arc<dyn ExternalValues>,
            )
        }
    };

    ExternalStage::new(label, compare, values).map_err(|err| anyhow!(err))
}

fn policy(configured: Option<PolicyConfig>, default: ExternalPolicy) -> ExternalPolicy {
    match configured {
        None => default,
        Some(PolicyConfig::Continue) => ExternalPolicy::Continue,
        Some(PolicyConfig::Skip) => ExternalPolicy::Skip,
        Some(PolicyConfig::Fail) => ExternalPolicy::Fail,
        Some(PolicyConfig::Replace) => ExternalPolicy::Replace,
    }
}

fn values_from_data(
    name: &str,
    key: &[String],
    data: &ExternalData,
) -> Result<Arc<dyn ExternalValues>> {
    match data {
        ExternalData::Records(records) => {
            let table = ExternalTable::from_records(key, records.clone())
                .map_err(|err| anyhow!("external values '{name}': {err}"))?;
            Ok(Arc::new(table))
        }
        ExternalData::Mapping(entries) => {
            let [field] = key else {
                return Err(anyhow!(
                    "external values '{name}' are keyed by one value, but the stage key has {} fields. Pass a list of records instead",
                    key.len()
                ));
            };
            let table = ExternalTable::from_mapping(field, entries.clone())
                .map_err(|err| anyhow!("external values '{name}': {err}"))?;
            Ok(Arc::new(table))
        }
        ExternalData::Lookup(values) => Ok(Arc::clone(values)),
    }
}

fn read_external_file(path: &Path, format: &str, sheet: Option<&str>) -> Result<Vec<Record>> {
    if sheet.is_some() && format != "excel" {
        return Err(anyhow!(
            "external file '{}' uses sheet, which only applies to format excel",
            path.display()
        ));
    }
    let source = source_for_format(path, format, sheet)?;
    let mut records = Vec::new();
    for record in source
        .read()
        .with_context(|| format!("Failed to read external values from {}", path.display()))?
    {
        records.push(
            record.with_context(|| {
                format!("Failed to read external values from {}", path.display())
            })?,
        );
    }
    Ok(records)
}

fn source_for_format(path: &Path, format: &str, sheet: Option<&str>) -> Result<Box<dyn Source>> {
    let path = path.to_path_buf();
    let config = match format {
        "json" => SourceConfig::Json { path },
        "json_array" => SourceConfig::JsonArray { path },
        "csv" => SourceConfig::Csv { path },
        "tsv" => SourceConfig::Tsv { path },
        "parquet" => SourceConfig::Parquet { path },
        "excel" => SourceConfig::Excel {
            path,
            sheet: sheet.map(str::to_string),
        },
        other => {
            return Err(anyhow!(
                "external format '{other}' is not one of json, json_array, csv, tsv, parquet, excel"
            ));
        }
    };
    Ok(build_source(&config))
}

fn infer_format(path: &Path) -> Result<String> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "jsonl" | "ndjson" => Ok("json".to_string()),
        "csv" => Ok("csv".to_string()),
        "tsv" | "tab" => Ok("tsv".to_string()),
        "parquet" => Ok("parquet".to_string()),
        "xlsx" | "xls" | "ods" => Ok("excel".to_string()),
        "json" => Err(anyhow!(
            "external file '{}' needs format: json or format: json_array",
            path.display()
        )),
        _ => Err(anyhow!(
            "external file '{}' needs a format. Use json, json_array, csv, tsv, parquet, or excel",
            path.display()
        )),
    }
}

fn string_or_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrList;

    impl<'de> Visitor<'de> for StringOrList {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a field name or a list of field names")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_string()])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut fields = Vec::new();
            while let Some(field) = seq.next_element()? {
                fields.push(field);
            }
            Ok(fields)
        }
    }

    deserializer.deserialize_any(StringOrList)
}
