use std::collections::HashMap;
use std::sync::Arc;

use satva_types::{Record, Value};

use crate::{PipelineStage, StageContext, StageError, StageResult};

/// What to do after comparing a row with its external values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalPolicy {
    /// Keep the row as it is.
    Continue,
    /// Drop the row.
    Skip,
    /// Reject the row.
    Fail,
    /// Copy the compared fields from the external values, then keep the row.
    Replace,
}

/// Fields and policies for one external comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCompare {
    pub key: Vec<String>,
    pub compare: Option<Vec<String>>,
    pub on_missing: ExternalPolicy,
    pub on_match: ExternalPolicy,
    pub on_differ: ExternalPolicy,
}

/// External values for one stage, supplied by the caller.
///
/// A list of records is indexed by the stage key. A mapping is a single key
/// value to the values to compare. A lookup is asked for each row.
#[derive(Clone)]
pub enum ExternalData {
    Records(Vec<Record>),
    Mapping(Vec<(Value, Record)>),
    Lookup(Arc<dyn ExternalValues>),
}

/// Resolves the external values for one key.
///
/// `key` contains only the key fields, in the stage's key order.
pub trait ExternalValues: Send + Sync {
    fn get(&self, key: &Record) -> Result<Option<Record>, String>;
}

/// External values indexed by one or more key fields.
#[derive(Debug, Clone)]
pub struct ExternalTable {
    key: Vec<String>,
    rows: HashMap<String, Record>,
}

impl ExternalTable {
    /// Index records that already contain `key` fields.
    pub fn from_records(key: &[String], records: Vec<Record>) -> Result<Self, String> {
        let mut table = Self {
            key: key.to_vec(),
            rows: HashMap::new(),
        };
        for record in records {
            let id = canonical_key(key, &record)?;
            if table.rows.contains_key(&id) {
                return Err(format!(
                    "duplicate external value for {}",
                    describe_key(key, &record)
                ));
            }
            table.rows.insert(id, record);
        }
        Ok(table)
    }

    /// Index values for a single key field. The key is stored beside the record.
    pub fn from_mapping(field: &str, entries: Vec<(Value, Record)>) -> Result<Self, String> {
        let key = vec![field.to_string()];
        let mut table = Self {
            key,
            rows: HashMap::new(),
        };
        for (value, record) in entries {
            if matches!(value, Value::Null) {
                return Err(format!("key field '{field}' is null"));
            }
            let mut key_record = Record::new();
            key_record.insert(field, value);
            let id = canonical_key(&[field.to_string()], &key_record)?;
            if table.rows.contains_key(&id) {
                return Err(format!(
                    "duplicate external value for {}",
                    describe_key(&[field.to_string()], &key_record)
                ));
            }
            table.rows.insert(id, record);
        }
        Ok(table)
    }
}

impl ExternalValues for ExternalTable {
    fn get(&self, key: &Record) -> Result<Option<Record>, String> {
        let id = canonical_key(&self.key, key)?;
        Ok(self.rows.get(&id).cloned())
    }
}

/// Compares each row with external values, then continues, skips, fails, or replaces fields.
pub struct ExternalStage {
    label: String,
    compare: ExternalCompare,
    values: Arc<dyn ExternalValues>,
}

impl ExternalStage {
    pub fn new(
        label: impl Into<String>,
        compare: ExternalCompare,
        values: Arc<dyn ExternalValues>,
    ) -> Result<Self, String> {
        if compare.key.is_empty() {
            return Err("external stage key must name at least one field".to_string());
        }
        if compare.key.iter().any(|field| field.is_empty()) {
            return Err("external stage key field must not be empty".to_string());
        }
        if compare
            .compare
            .as_ref()
            .is_some_and(|fields| fields.iter().any(|field| field.is_empty()))
        {
            return Err("external stage compare field must not be empty".to_string());
        }
        if compare.on_missing == ExternalPolicy::Replace {
            return Err(
                "external stage on_missing cannot be replace, because there is no external value"
                    .to_string(),
            );
        }
        Ok(Self {
            label: label.into(),
            compare,
            values,
        })
    }

    fn fields_to_copy<'a>(&'a self, external: &'a Record) -> Vec<&'a str> {
        match &self.compare.compare {
            Some(fields) => fields.iter().map(String::as_str).collect(),
            None => external
                .keys()
                .filter(|field| !self.compare.key.iter().any(|key| key == *field))
                .map(String::as_str)
                .collect(),
        }
    }
}

impl PipelineStage for ExternalStage {
    fn name(&self) -> &'static str {
        "external"
    }

    fn execute(&self, record: &mut Record, _ctx: &StageContext) -> StageResult {
        let key = match key_record(record, &self.compare.key) {
            Ok(key) => key,
            Err(message) => return fail(message),
        };
        let found = match self.values.get(&key) {
            Ok(found) => found,
            Err(message) => return fail(format!("{}: {message}", self.label)),
        };
        let Some(external) = found else {
            return apply(
                self.compare.on_missing,
                record,
                None,
                &[],
                &format!(
                    "no external value for {}",
                    describe_key(&self.compare.key, &key)
                ),
            );
        };

        let fields = self.fields_to_copy(&external);
        let same = fields
            .iter()
            .all(|field| field_value(record, field) == field_value(&external, field));
        let described = describe_key(&self.compare.key, &key);
        if same {
            apply(
                self.compare.on_match,
                record,
                Some(&external),
                &fields,
                &format!("matches external value for {described}"),
            )
        } else {
            apply(
                self.compare.on_differ,
                record,
                Some(&external),
                &fields,
                &format!("differs from external value for {described}"),
            )
        }
    }
}

fn apply(
    policy: ExternalPolicy,
    record: &mut Record,
    external: Option<&Record>,
    fields: &[&str],
    reason: &str,
) -> StageResult {
    match policy {
        ExternalPolicy::Continue => StageResult::continue_(),
        ExternalPolicy::Skip => StageResult::skip(reason.to_string()),
        ExternalPolicy::Fail => fail(reason.to_string()),
        ExternalPolicy::Replace => {
            let Some(external) = external else {
                return fail(reason.to_string());
            };
            for field in fields {
                record.insert(field, field_value(external, field));
            }
            StageResult::continue_()
        }
    }
}

fn fail(message: impl Into<String>) -> StageResult {
    StageResult::fail(StageError::execution("external", message))
}

fn key_record(record: &Record, key: &[String]) -> Result<Record, String> {
    let mut out = Record::new();
    for field in key {
        match record.get(field) {
            Some(Value::Null) | None => {
                return Err(format!("missing key field '{field}'"));
            }
            Some(value) => out.insert(field, value.clone()),
        }
    }
    Ok(out)
}

fn field_value(record: &Record, field: &str) -> Value {
    record.get(field).cloned().unwrap_or(Value::Null)
}

fn canonical_key(key: &[String], record: &Record) -> Result<String, String> {
    let mut encoded = String::new();
    for field in key {
        let value = record
            .get(field)
            .ok_or_else(|| format!("missing key field '{field}'"))?;
        if matches!(value, Value::Null) {
            return Err(format!("key field '{field}' is null"));
        }
        encoded.push('\u{1f}');
        encoded.push_str(&encode_value(value));
    }
    Ok(encoded)
}

fn encode_value(value: &Value) -> String {
    match value {
        Value::Null => "n".to_string(),
        Value::Int64(value) => format!("i{value}"),
        Value::Float64(value) => format!("f{value}"),
        Value::Boolean(value) => format!("b{value}"),
        Value::String(value) => format!("s{value}"),
    }
}

fn describe_key(key: &[String], record: &Record) -> String {
    key.iter()
        .map(|field| {
            let value = record.get(field).cloned().unwrap_or(Value::Null);
            format!("{field}={value}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}
