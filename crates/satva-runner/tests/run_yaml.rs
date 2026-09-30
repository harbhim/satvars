use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use satva_core::{ErrorPolicy, PipelineOptions};
use satva_runner::run_yaml;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("satva-runner-{nanos}"));
        fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_pipeline(dir: &Path, stages: &str) -> PathBuf {
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
            "source:\n  type: json\n  path: {}\nsink:\n  type: json\n  path: {}\nstages:\n{stages}\n",
            input.display(),
            output.display(),
        ),
    )
    .expect("write config");
    config
}

#[test]
fn run_yaml_filters_and_writes_the_sink() {
    let dir = TempDir::new();
    let config = write_pipeline(
        dir.path(),
        "  - type: filter\n    expression: 'active == true && salary >= 70000'",
    );

    let report = run_yaml(&config, PipelineOptions::new()).expect("run");

    assert!(report.schema.is_none());
    assert_eq!(report.summary.processed, 2);
    assert_eq!(report.summary.succeeded, 1);
    assert_eq!(report.summary.skipped, 1);
    assert_eq!(report.summary.failed, 0);

    let output = fs::read_to_string(dir.path().join("output.jsonl")).expect("read output");
    assert!(output.contains("Ada"));
    assert!(!output.contains("Ben"));
}

#[test]
fn run_yaml_reads_tsv_and_writes_json_array() {
    let dir = TempDir::new();
    let input = dir.path().join("input.tsv");
    let output = dir.path().join("output.json");
    fs::write(&input, "name\tage\nAda\t36\nBen\t41\n").expect("write input");
    let config = dir.path().join("pipeline.yaml");
    fs::write(
        &config,
        format!(
            "source:\n  type: tsv\n  path: {}\nsink:\n  type: json_array\n  path: {}\nstages:\n  - type: filter\n    expression: 'age == \"36\"'\n",
            input.display(),
            output.display(),
        ),
    )
    .expect("write config");

    let report = run_yaml(&config, PipelineOptions::new()).expect("run");
    assert_eq!(report.summary.succeeded, 1);

    let written = fs::read_to_string(output).expect("read output");
    assert!(written.contains("Ada"));
    assert!(!written.contains("Ben"));
}

#[test]
fn run_yaml_stop_on_error_returns_the_record_failure() {
    let dir = TempDir::new();
    let config = write_pipeline(
        dir.path(),
        "  - type: set_field\n    field: bonus\n    expression: '1 / 0'",
    );

    let error = run_yaml(
        &config,
        PipelineOptions::new().with_error_policy(ErrorPolicy::StopOnError),
    )
    .expect_err("division by zero fails the run");

    let message = format!("{error:#}");
    assert!(
        message.contains("Record 1 failed"),
        "unexpected error: {message}"
    );
    assert!(
        message.contains("division by zero"),
        "unexpected error: {message}"
    );
}
