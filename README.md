# Satva

Satva is a modular data pipeline engine written in Rust. Read records from CSV or JSONL, apply validation and transformation stages, and write the results to CSV or JSONL. Pipelines can be built with the Rust libraries or configured in YAML and run from the command line.

## Features

- CSV and JSONL sources and sinks.
- Schema inference and validation with type coercion.
- Filtering, computed fields, field renaming, field selection, and field removal.
- An expression language with arithmetic, comparisons, boolean operators, and built-in functions.
- Pipeline summaries and per-record logs.

Stages currently execute sequentially. The Arrow, parallel execution, and Python crates are placeholders.

## Quick start

Install a current stable Rust toolchain with Cargo and support for the Rust 2024 edition. Run the following commands from the repository root.

Build the CLI:

```bash
cargo build --release -p satva-cli
```

Create a small input file and pipeline configuration:

```bash
mkdir -p tmp

cat > tmp/input.jsonl <<'EOF'
{"name":"Ada","active":true,"salary":80000}
{"name":"Ben","active":false,"salary":60000}
EOF

cat > tmp/pipeline.yaml <<'EOF'
source:
  type: json
  path: tmp/input.jsonl

sink:
  type: json
  path: tmp/output.jsonl

stages:
  - type: filter
    expression: 'active == true && salary >= 70000'
  - type: set_field
    field: bonus
    expression: 'salary * 0.15'
  - type: select_fields
    fields: [name, salary, bonus]
EOF

cargo run -p satva-cli -- run --config tmp/pipeline.yaml
cat tmp/output.jsonl
```

The output contains Ada's record with a bonus of `12000.0`; Ben's record is filtered out. The CLI also prints a pipeline summary and any logs.

File paths in the YAML configuration are relative to the process's working directory. The `json` format is JSONL: one JSON object per line. CSV inputs initially contain string values; use schema inference and a `schema_validation` stage when type coercion is needed.

A more detailed [employee pipeline](crates/satva-cli/examples/pipeline.yaml) demonstrates schema validation and string expressions. It requires an `employees.jsonl` input file, which is not included in the repository.

## Workspace layout

| Crate | Purpose |
| --- | --- |
| `satva-types` | Records, values, schemas, and data types |
| `satva-expr` | Expression builders and evaluation |
| `satva-parser` | Parsing expression strings |
| `satva-core` | Pipeline orchestration and built-in stages |
| `satva-io` | CSV and JSONL readers and writers |
| `satva-cli` | YAML-driven command-line interface |
| `satva-arrow` | Placeholder for Arrow interoperability |
| `satva-execution` | Placeholder for parallel execution |
| `satva-python` | Placeholder for Python bindings |
| `satva-benchmarks` | Criterion benchmarks in `benchmarks/` |

## Development

```bash
# Run all workspace tests
cargo test --workspace --all-features

# Check formatting and lint all targets
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run benchmarks
cargo bench -p satva-benchmarks
```

The comprehensive check script, `bash scripts/check.sh`, also runs `cargo audit` and a release build. It requires `cargo-audit` to be installed separately.

## Documentation

- [Getting started and Rust library usage](docs/getting-started.md)
- [Pipeline configuration](docs/pipeline-config.md)
- [Expression language](docs/expression-language.md)
- [Architecture](docs/architecture.md)

### Reliable pipeline completion

`Pipeline::run` calls the sink's fallible `finish()` hook before returning, even
when a source, stage, or sink write fails. JSON and CSV sinks flush their buffered
output there. A finish error always makes the run fail. If processing and finishing
both fail, the returned error chain includes both errors. Custom sinks inherit a
no-op hook and should override it if they buffer output. Finishing does not roll
back partial output or guarantee that data has been synced to disk.

Record failures continue by default for compatibility. Integrations that treat
`Ok` as a successful sync should select `StopOnError`:

```rust,ignore
use satva_core::{ErrorPolicy, PipelineOptions};

let result = pipeline.run(
    PipelineOptions::default()
        .with_error_policy(ErrorPolicy::StopOnError)
        .with_log_limit(Some(1_000)),
)?;
```

`StopOnError` stops at the first stage or sink-write failure and returns its error
with the record index, even when logs are disabled or full. Intentional skips are
not failures. With `ErrorPolicy::Continue`, check `result.summary.failed` before
marking a sync successful. Source errors always stop processing.

By default, only the first 1,000 skipped/failed record logs are retained; summary
counts still cover every processed record. Use `with_log_limit(Some(n))` to change
the cap, `with_log_limit(None)` for unlimited logs, or `without_logs()` to disable
retention. Existing `PipelineOptions` struct literals need
`..PipelineOptions::default()` to initialize the new fields.

Integer arithmetic now returns evaluation errors for overflow, division or
remainder by zero, and negation of `i64::MIN`. Floating-point arithmetic (including
mixed integer/float operations and negation) rejects non-finite results such as
NaN and infinity. Finite results, including underflow to zero, remain valid. This
arithmetic rule does not change literal, field, or cast behavior.
