use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use satva_core::PipelineOptions;
use satva_runner::{PipelineConfig, PipelineRunReport, run_config};

/// Run `config_path`, reading the source from a Django file object.
///
/// `source` is the value of a model `FileField` (`FieldFile`) or another
/// `django.core.files.File`. The file name must use a data extension that
/// matches the YAML `source.type`. Bytes are read from the file object, so
/// storage backends without a local path still work. `source.path` in the
/// YAML is replaced for this run.
pub fn run_with_django_source(
    config_path: &Path,
    options: PipelineOptions,
    source: &Bound<'_, PyAny>,
) -> PyResult<PipelineRunReport> {
    if !is_django_file(source)? {
        return Err(PyRuntimeError::new_err(
            "source must be a Django FileField value or a django.core.files.File",
        ));
    }

    let mut config = PipelineConfig::load(config_path)
        .map_err(|err| PyRuntimeError::new_err(format!("{err:#}")))?;
    let name = django_file_name(source)?;
    ensure_data_file(&name, config.source.kind())?;
    let bytes = read_django_file(source)?;
    let temp = TempDataFile::create(&name, &bytes)?;
    config.source.set_path(temp.path.clone());

    run_config(config, options).map_err(|err| PyRuntimeError::new_err(format!("{err:#}")))
}

fn is_django_file(obj: &Bound<'_, PyAny>) -> PyResult<bool> {
    if type_matches(obj, is_django_file_class)? {
        return Ok(true);
    }
    has_file_field(obj)
}

fn has_file_field(obj: &Bound<'_, PyAny>) -> PyResult<bool> {
    if !obj.hasattr("field")? {
        return Ok(false);
    }
    let field = obj.getattr("field")?;
    if field.is_none() {
        return Ok(false);
    }
    type_matches(&field, is_django_field_class)
}

fn is_saved_field_file(obj: &Bound<'_, PyAny>) -> PyResult<bool> {
    if type_matches(obj, is_field_file_class)? {
        return Ok(true);
    }
    has_file_field(obj)
}

fn is_field_file_class(module: &str, name: &str) -> bool {
    module.starts_with("django.db.models.fields.files")
        && matches!(name, "FieldFile" | "ImageFieldFile")
}

fn is_django_file_class(module: &str, name: &str) -> bool {
    let django_file = module.starts_with("django.core.files")
        || module.starts_with("django.db.models.fields.files");
    django_file
        && matches!(
            name,
            "FieldFile"
                | "ImageFieldFile"
                | "File"
                | "ContentFile"
                | "UploadedFile"
                | "InMemoryUploadedFile"
                | "TemporaryUploadedFile"
                | "SimpleUploadedFile"
        )
}

fn is_django_field_class(module: &str, name: &str) -> bool {
    module.starts_with("django.db.models") && matches!(name, "FileField" | "ImageField")
}

fn type_matches(obj: &Bound<'_, PyAny>, pred: impl Fn(&str, &str) -> bool) -> PyResult<bool> {
    let mro = obj.get_type().getattr("__mro__")?;
    let mro = mro.downcast::<PyTuple>()?;
    for cls in mro.iter() {
        let module = cls.getattr("__module__")?.extract::<String>()?;
        let name = cls.getattr("__name__")?.extract::<String>()?;
        if pred(&module, &name) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn django_file_name(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let name = obj
        .getattr("name")
        .map_err(|_| PyRuntimeError::new_err("Django FileField has no file name"))?;
    if name.is_none() {
        return Err(PyRuntimeError::new_err("Django FileField is empty"));
    }
    let name: String = name
        .extract()
        .map_err(|_| PyRuntimeError::new_err("Django FileField name must be a string"))?;
    if name.is_empty() {
        return Err(PyRuntimeError::new_err("Django FileField is empty"));
    }
    Ok(name)
}

fn ensure_data_file(name: &str, kind: &str) -> PyResult<()> {
    let ext = extension(name);
    if !is_data_extension(&ext) {
        return Err(PyRuntimeError::new_err(format!(
            "Django file '{name}' is not a data file. Supported extensions: csv, tsv, tab, json, jsonl, ndjson, parquet, xlsx, xls, ods"
        )));
    }
    if !extension_matches(kind, &ext) {
        return Err(PyRuntimeError::new_err(format!(
            "Django file '{name}' does not match source type '{kind}' in the YAML config"
        )));
    }
    Ok(())
}

fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_data_extension(ext: &str) -> bool {
    matches!(
        ext,
        "csv" | "tsv" | "tab" | "json" | "jsonl" | "ndjson" | "parquet" | "xlsx" | "xls" | "ods"
    )
}

fn extension_matches(kind: &str, ext: &str) -> bool {
    match kind {
        "csv" => ext == "csv",
        "tsv" => ext == "tsv" || ext == "tab",
        "json" => ext == "json" || ext == "jsonl" || ext == "ndjson",
        "json_array" => ext == "json",
        "parquet" => ext == "parquet",
        "excel" => ext == "xlsx" || ext == "xls" || ext == "ods",
        _ => false,
    }
}

fn read_django_file(obj: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    // A saved FileField reads through storage. An uploaded file already has
    // its bytes; opening it again looks for `name` on the local disk.
    let opened = is_saved_field_file(obj)?;
    if opened {
        obj.call_method1("open", ("rb",))?;
    }
    let _guard = CloseFile { obj, close: opened };

    if obj.hasattr("seek")? {
        let _ = obj.call_method1("seek", (0,));
    }

    let data = obj.call_method0("read")?;
    if let Ok(bytes) = data.extract::<Vec<u8>>() {
        return Ok(bytes);
    }
    if let Ok(text) = data.extract::<String>() {
        return Ok(text.into_bytes());
    }
    Err(PyRuntimeError::new_err(
        "Django file read() must return bytes or text",
    ))
}

struct CloseFile<'a> {
    obj: &'a Bound<'a, PyAny>,
    close: bool,
}

impl Drop for CloseFile<'_> {
    fn drop(&mut self) {
        if self.close {
            let _ = self.obj.call_method0("close");
        }
    }
}

struct TempDataFile {
    path: PathBuf,
}

impl TempDataFile {
    fn create(filename: &str, bytes: &[u8]) -> PyResult<Self> {
        let ext = extension(filename);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| PyRuntimeError::new_err(format!("system clock error: {err}")))?
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("satva-django-{}-{nanos}.{ext}", std::process::id()));
        fs::write(&path, bytes).map_err(|err| {
            PyRuntimeError::new_err(format!("Failed to buffer Django file: {err}"))
        })?;
        Ok(Self { path })
    }
}

impl Drop for TempDataFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
