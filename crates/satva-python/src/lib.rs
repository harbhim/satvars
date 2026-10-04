mod django;
mod external;

use std::path::Path;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use satva_core::{ErrorPolicy, PipelineLog, PipelineOptions};
use satva_runner::{run_config_with, run_yaml};

/// Run a YAML pipeline.
///
/// Returns a dict with `processed`, `succeeded`, `skipped`, `failed`, and `logs`.
/// `stop_on_error` uses `ErrorPolicy::StopOnError` and raises `RuntimeError` on the
/// first record failure. File paths inside the config are relative to the process
/// working directory.
///
/// `source` accepts a Django `FileField` value or `django.core.files.File`. Satva
/// reads that file object when its name is a data file whose extension matches
/// `source.type` in the YAML. The YAML `source.path` is not used in that case.
///
/// `externals` maps each named YAML `external` stage to the values it compares
/// against: a list of records, a key-to-values mapping, or a lookup callable.
/// The stage decides whether to continue, skip, fail, or replace.
#[pyfunction]
#[pyo3(signature = (path, *, stop_on_error = false, source = None, externals = None))]
fn run(
    py: Python<'_>,
    path: &str,
    stop_on_error: bool,
    source: Option<Bound<'_, PyAny>>,
    externals: Option<Bound<'_, PyAny>>,
) -> PyResult<Py<PyDict>> {
    let options = if stop_on_error {
        PipelineOptions::new().with_error_policy(ErrorPolicy::StopOnError)
    } else {
        PipelineOptions::new()
    };

    let provided = external::data_from_py(externals.as_ref())?;
    let report = match source {
        Some(source) => {
            django::run_with_django_source(Path::new(path), options, &source, &provided)?
        }
        None => if provided.is_empty() {
            run_yaml(Path::new(path), options)
        } else {
            let config = satva_runner::PipelineConfig::load(Path::new(path))
                .map_err(|err| PyRuntimeError::new_err(format!("{err:#}")))?;
            run_config_with(config, options, &provided)
        }
        .map_err(|err| PyRuntimeError::new_err(format!("{err:#}")))?,
    };

    let dict = PyDict::new(py);
    dict.set_item("processed", report.summary.processed)?;
    dict.set_item("succeeded", report.summary.succeeded)?;
    dict.set_item("skipped", report.summary.skipped)?;
    dict.set_item("failed", report.summary.failed)?;

    let logs = PyList::empty(py);
    for log in &report.logs {
        logs.append(format_log(log))?;
    }
    dict.set_item("logs", logs)?;

    Ok(dict.unbind())
}

fn format_log(log: &PipelineLog) -> String {
    match log {
        PipelineLog::Skipped {
            record_index,
            stage,
            reason,
        } => format!("skipped record {record_index} at {stage}: {reason}"),
        PipelineLog::StageFailed {
            record_index,
            error,
        } => {
            format!("record {record_index} failed: {error}")
        }
        PipelineLog::SinkFailed {
            record_index,
            message,
        } => format!("record {record_index} sink failed: {message}"),
    }
}

#[pymodule]
fn satva(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(run, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("satva-python-{nanos}"));
            fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_pipeline(dir: &Path) -> PathBuf {
        let input = dir.join("input.jsonl");
        let output = dir.join("output.jsonl");
        fs::write(
            &input,
            "{\"name\":\"Ada\",\"active\":true,\"salary\":80000}\n{\"name\":\"Ben\",\"active\":false,\"salary\":60000}\n",
        )
        .expect("write input");
        let config = dir.join("pipeline.yaml");
        fs::write(
            &config,
            format!(
                "source:\n  type: json\n  path: {}\nsink:\n  type: json\n  path: {}\nstages:\n  - type: filter\n    expression: 'active == true && salary >= 70000'\n",
                input.display(),
                output.display(),
            ),
        )
        .expect("write config");
        config
    }

    #[test]
    fn run_returns_summary_counts() {
        Python::initialize();
        let dir = TempDir::new();
        let config = write_pipeline(&dir.0);

        Python::attach(|py| {
            let summary =
                run(py, config.to_str().expect("utf8 path"), false, None, None).expect("run");
            let summary = summary.bind(py);
            assert_eq!(
                summary
                    .get_item("processed")
                    .expect("item")
                    .expect("processed")
                    .extract::<usize>()
                    .expect("usize"),
                2
            );
            assert_eq!(
                summary
                    .get_item("succeeded")
                    .expect("item")
                    .expect("succeeded")
                    .extract::<usize>()
                    .expect("usize"),
                1
            );
            assert_eq!(
                summary
                    .get_item("skipped")
                    .expect("item")
                    .expect("skipped")
                    .extract::<usize>()
                    .expect("usize"),
                1
            );
            assert_eq!(
                summary
                    .get_item("failed")
                    .expect("item")
                    .expect("failed")
                    .extract::<usize>()
                    .expect("usize"),
                0
            );
        });
    }

    #[test]
    fn stop_on_error_raises() {
        Python::initialize();
        let dir = TempDir::new();
        let input = dir.0.join("input.jsonl");
        let output = dir.0.join("output.jsonl");
        fs::write(&input, "{\"salary\":1}\n").expect("write input");
        let config = dir.0.join("pipeline.yaml");
        fs::write(
            &config,
            format!(
                "source:\n  type: json\n  path: {}\nsink:\n  type: json\n  path: {}\nstages:\n  - type: set_field\n    field: bonus\n    expression: '1 / 0'\n",
                input.display(),
                output.display(),
            ),
        )
        .expect("write config");

        Python::attach(|py| {
            let error =
                run(py, config.to_str().expect("utf8 path"), true, None, None).expect_err("raises");
            assert!(error.is_instance_of::<PyRuntimeError>(py));
            let message = error.to_string();
            assert!(message.contains("Record 1 failed"), "{message}");
        });
    }

    fn django_file<'py>(py: Python<'py>, name: &str, data: &[u8]) -> PyResult<Bound<'py, PyAny>> {
        let locals = PyDict::new(py);
        locals.set_item("file_name", name)?;
        locals.set_item("file_data", data)?;
        py.run(
            c"
class FileField:
    pass
FileField.__module__ = 'django.db.models.fields.files'

class FieldFile:
    def __init__(self, name, data):
        self.field = FileField()
        self.name = name
        self.data = data
        self.read_called = False
        self.opened_mode = None
        self.closed = False
    def open(self, mode='rb'):
        self.opened_mode = mode
        return self
    def read(self):
        self.read_called = True
        return self.data
    def close(self):
        self.closed = True

uploaded = FieldFile(file_name, file_data)
",
            Some(&locals),
            Some(&locals),
        )?;
        locals
            .get_item("uploaded")?
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("missing uploaded"))
    }

    #[test]
    fn django_file_field_is_read_as_the_source() {
        Python::initialize();
        let dir = TempDir::new();
        let output = dir.0.join("output.jsonl");
        let config = dir.0.join("pipeline.yaml");
        fs::write(
            &config,
            format!(
                "source:\n  type: json\n  path: ignored.jsonl\nsink:\n  type: json\n  path: {}\nstages:\n  - type: filter\n    expression: 'active == true'\n",
                output.display(),
            ),
        )
        .expect("write config");

        Python::attach(|py| {
            let file = django_file(
                py,
                "uploads/people.jsonl",
                b"{\"name\":\"Ada\",\"active\":true}\n{\"name\":\"Ben\",\"active\":false}\n",
            )
            .expect("file");
            let summary = run(
                py,
                config.to_str().expect("utf8 path"),
                false,
                Some(file.clone()),
                None,
            )
            .expect("run");
            let summary = summary.bind(py);
            assert_eq!(
                summary
                    .get_item("succeeded")
                    .expect("item")
                    .expect("succeeded")
                    .extract::<usize>()
                    .expect("usize"),
                1
            );
            assert!(
                file.getattr("read_called")
                    .expect("attr")
                    .extract::<bool>()
                    .expect("bool")
            );
            assert_eq!(
                file.getattr("opened_mode")
                    .expect("attr")
                    .extract::<String>()
                    .expect("mode"),
                "rb"
            );
            assert!(
                file.getattr("closed")
                    .expect("attr")
                    .extract::<bool>()
                    .expect("bool")
            );
        });

        let written = fs::read_to_string(output).expect("read output");
        assert!(written.contains("Ada"));
        assert!(!written.contains("Ben"));
    }

    #[test]
    fn django_uploaded_file_is_read_without_reopening() {
        Python::initialize();
        let dir = TempDir::new();
        let output = dir.0.join("output.jsonl");
        let config = dir.0.join("pipeline.yaml");
        fs::write(
            &config,
            format!(
                "source:\n  type: json\n  path: ignored.jsonl\nsink:\n  type: json\n  path: {}\nstages: []\n",
                output.display(),
            ),
        )
        .expect("write config");

        Python::attach(|py| {
            let locals = PyDict::new(py);
            locals
                .set_item("file_data", b"{\"name\":\"Ada\"}\n".as_slice())
                .expect("data");
            py.run(
                c"
class UploadedFile:
    def __init__(self, data):
        self.name = 'people.jsonl'
        self.data = data
        self.read_called = False
    def read(self):
        self.read_called = True
        return self.data
UploadedFile.__module__ = 'django.core.files.uploadedfile'
uploaded = UploadedFile(file_data)
",
                Some(&locals),
                Some(&locals),
            )
            .expect("define upload");
            let file = locals
                .get_item("uploaded")
                .expect("item")
                .expect("uploaded");
            run(
                py,
                config.to_str().expect("utf8 path"),
                false,
                Some(file.clone()),
                None,
            )
            .expect("run");
            assert!(
                file.getattr("read_called")
                    .expect("attr")
                    .extract::<bool>()
                    .expect("bool")
            );
        });

        let written = fs::read_to_string(output).expect("read output");
        assert!(written.contains("Ada"));
    }

    #[test]
    fn django_file_field_rejects_a_non_data_file() {
        Python::initialize();
        let dir = TempDir::new();
        let config = dir.0.join("pipeline.yaml");
        fs::write(
            &config,
            "source:\n  type: json\n  path: ignored.jsonl\nstages: []\n",
        )
        .expect("write config");

        Python::attach(|py| {
            let file = django_file(py, "uploads/photo.png", b"not-an-image").expect("file");
            let error = run(
                py,
                config.to_str().expect("utf8 path"),
                false,
                Some(file),
                None,
            )
            .expect_err("rejects");
            let message = error.to_string();
            assert!(message.contains("not a data file"), "{message}");
        });
    }

    #[test]
    fn django_file_field_requires_the_yaml_source_type() {
        Python::initialize();
        let dir = TempDir::new();
        let config = dir.0.join("pipeline.yaml");
        fs::write(
            &config,
            "source:\n  type: csv\n  path: ignored.csv\nstages: []\n",
        )
        .expect("write config");

        Python::attach(|py| {
            let file = django_file(py, "uploads/people.jsonl", b"{}\n").expect("file");
            let error = run(
                py,
                config.to_str().expect("utf8 path"),
                false,
                Some(file),
                None,
            )
            .expect_err("rejects");
            let message = error.to_string();
            assert!(
                message.contains("does not match source type 'csv'"),
                "{message}"
            );
        });
    }

    fn summary_count(summary: &Bound<'_, PyDict>, key: &str) -> usize {
        summary
            .get_item(key)
            .expect("item")
            .expect(key)
            .extract::<usize>()
            .expect("usize")
    }

    #[test]
    fn external_values_skip_matches_and_replace_differences() {
        Python::initialize();
        let dir = TempDir::new();
        let input = dir.0.join("products.jsonl");
        let output = dir.0.join("output.jsonl");
        fs::write(
            &input,
            concat!(
                "{\"sku\":\"new\",\"name\":\"Widget\",\"price\":10}\n",
                "{\"sku\":\"same\",\"name\":\"Same\",\"price\":5}\n",
                "{\"sku\":\"changed\",\"name\":\"Old\",\"price\":1}\n",
            ),
        )
        .expect("write input");
        let config = dir.0.join("pipeline.yaml");
        fs::write(
            &config,
            format!(
                "source:\n  type: json\n  path: {}\nsink:\n  type: json\n  path: {}\nstages:\n  - type: external\n    name: catalog\n    key: sku\n    compare: [name, price]\n",
                input.display(),
                output.display(),
            ),
        )
        .expect("write config");

        Python::attach(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"
catalog = [
    {'sku': 'same', 'name': 'Same', 'price': 5},
    {'sku': 'changed', 'name': 'FromDb', 'price': 9},
]
",
                None,
                Some(&locals),
            )
            .expect("define catalog");
            let externals = PyDict::new(py);
            externals
                .set_item(
                    "catalog",
                    locals.get_item("catalog").expect("item").expect("catalog"),
                )
                .expect("externals");

            let summary = run(
                py,
                config.to_str().expect("utf8 path"),
                false,
                None,
                Some(externals.into_any()),
            )
            .expect("run");
            let summary = summary.bind(py);
            assert_eq!(summary_count(&summary, "processed"), 3);
            assert_eq!(summary_count(&summary, "succeeded"), 2);
            assert_eq!(summary_count(&summary, "skipped"), 1);
            assert_eq!(summary_count(&summary, "failed"), 0);
            let logs = summary.get_item("logs").expect("item").expect("logs");
            let logs = logs.extract::<Vec<String>>().expect("log strings");
            assert!(logs.iter().any(|line| line.contains("sku=same")));
        });

        let written = fs::read_to_string(output).expect("read output");
        assert!(written.contains("Widget"));
        assert!(written.contains("FromDb"));
        assert!(!written.contains("Same"));
        assert!(!written.contains("\"Old\""));
    }

    #[test]
    fn external_stage_requires_values() {
        Python::initialize();
        let dir = TempDir::new();
        let input = dir.0.join("products.jsonl");
        fs::write(&input, "{\"sku\":\"new\"}\n").expect("write input");
        let config = dir.0.join("pipeline.yaml");
        fs::write(
            &config,
            format!(
                "source:\n  type: json\n  path: {}\nstages:\n  - type: external\n    name: catalog\n    key: sku\n",
                input.display(),
            ),
        )
        .expect("write config");

        Python::attach(|py| {
            let error = run(py, config.to_str().expect("utf8 path"), false, None, None)
                .expect_err("missing values");
            let message = error.to_string();
            assert!(message.contains("catalog"), "{message}");
            assert!(message.contains("no values"), "{message}");
        });
    }

    #[test]
    fn django_source_can_compare_lookup_values() {
        Python::initialize();
        let dir = TempDir::new();
        let output = dir.0.join("output.jsonl");
        let config = dir.0.join("pipeline.yaml");
        fs::write(
            &config,
            format!(
                "source:\n  type: json\n  path: ignored.jsonl\nsink:\n  type: json\n  path: {}\nstages:\n  - type: external\n    name: catalog\n    key: sku\n    compare: [name]\n",
                output.display(),
            ),
        )
        .expect("write config");

        Python::attach(|py| {
            let file = django_file(
                py,
                "uploads/products.jsonl",
                b"{\"sku\":\"same\",\"name\":\"Same\"}\n{\"sku\":\"new\",\"name\":\"Widget\"}\n",
            )
            .expect("file");
            let locals = PyDict::new(py);
            py.run(
                c"
def catalog(key):
    if key['sku'] == 'same':
        return {'name': 'Same'}
    return None
",
                None,
                Some(&locals),
            )
            .expect("define lookup");
            let externals = PyDict::new(py);
            externals
                .set_item(
                    "catalog",
                    locals.get_item("catalog").expect("item").expect("lookup"),
                )
                .expect("externals");
            let summary = run(
                py,
                config.to_str().expect("utf8 path"),
                false,
                Some(file),
                Some(externals.into_any()),
            )
            .expect("run");
            let summary = summary.bind(py);
            assert_eq!(summary_count(&summary, "succeeded"), 1);
            assert_eq!(summary_count(&summary, "skipped"), 1);
        });

        let written = fs::read_to_string(output).expect("read output");
        assert!(written.contains("Widget"));
        assert!(!written.contains("Same"));
    }
}
