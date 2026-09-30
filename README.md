# Satva

Satva is a data pipeline engine written in Rust. It reads CSV, TSV, JSONL, JSON arrays, Parquet, or Excel, runs validation and transformation stages, and writes the same formats. Each record continues, is skipped, or fails. The same YAML config runs from the command line, from Rust, and from Python.

## Features

- CSV, TSV, JSONL, JSON array, Parquet, and Excel sources and sinks.
- Schema inference and validation with type coercion.
- Filter, computed fields, rename, select, and remove stages.
- An expression language with arithmetic, comparisons, short-circuit boolean operators, and built-in functions (`upper`, `lower`, `trim`, `length`, `concat`, `coalesce`, `is_null`, `is_not_null`, and casts).
- Pipeline summaries and per-record logs.
- `Continue` and `StopOnError` policies. `Continue` is the default. `StopOnError` returns the first record failure.
- A shared YAML runner (`satva-runner`) used by the CLI and the Python extension.
- Columnar batches with vectorized filter, select, and computed-field stages. `satva-execution` runs those stages across batches in parallel. Row stages still run one record at a time.

## Quick start

Install a current stable Rust toolchain with Cargo and support for the Rust 2024 edition. Run the commands below from the repository root.

```bash
cargo build --release -p satva-cli

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

The output contains Ada's record with a bonus of `12000.0`. Ben's record is filtered out. The CLI prints a pipeline summary and any logs.

File paths in the YAML configuration are relative to the process working directory. The `json` format is JSONL: one JSON object per line. `json_array` is one JSON array of objects. CSV and TSV inputs are read as strings. Use schema inference and a `schema_validation` stage when those strings need to be coerced. Parquet and Excel sinks buffer rows until the run finishes.

A more detailed [employee pipeline](crates/satva-cli/examples/pipeline.yaml) demonstrates schema validation and string expressions. It expects an `employees.jsonl` input file, which is not included in the repository.

## Python

`satva.run` calls the same runner as the CLI. Build the extension from `crates/satva-python` with [maturin](https://www.maturin.rs/):

```bash
pip install maturin
cd crates/satva-python
maturin develop
```

```python
import satva

summary = satva.run("pipeline.yaml")
print(summary["processed"], summary["succeeded"], summary["skipped"], summary["failed"])

# Raises RuntimeError on the first record failure.
satva.run("pipeline.yaml", stop_on_error=True)
```

The returned dict contains `processed`, `succeeded`, `skipped`, `failed`, and `logs`. `logs` is a list of strings. `stop_on_error` defaults to `False`, which matches the CLI: failed records are counted and the call returns. The package requires Python 3.9 or newer. It is not published to PyPI.

## Rust

```rust,ignore
use satva_core::PipelineOptions;
use satva_runner::run_yaml;

let report = run_yaml("pipeline.yaml", PipelineOptions::new())?;
println!("{:?}", report.summary);
```

`report.schema` is set when the config asks for schema inference. `report.logs` holds skipped and failed records, up to the log limit. To build stages in code instead of YAML, see [Getting started](docs/getting-started.md).

## Run behavior

`Pipeline::run` calls the sink's `finish()` hook before returning, including when a source, stage, or sink write fails. JSON and CSV sinks flush buffered output there. JSON array, Parquet, and Excel sinks write their buffered result there. A finish error fails the run. If processing and finishing both fail, the returned error includes both. Custom sinks inherit a no-op hook and should override it when they buffer output. Finishing does not roll back partial output or sync data to disk.

Record failures continue by default. A caller that treats `Ok` as a successful sync should use `StopOnError`:

```rust,ignore
use satva_core::{ErrorPolicy, PipelineOptions};

let result = pipeline.run(
    PipelineOptions::default()
        .with_error_policy(ErrorPolicy::StopOnError)
        .with_log_limit(Some(1_000)),
)?;
```

`StopOnError` stops at the first stage or sink-write failure and returns that error with the record index, even when logs are disabled or full. Skips are not failures. With `ErrorPolicy::Continue`, check `result.summary.failed` before treating the sync as successful. Source errors always stop processing.

Only the first 1,000 skipped or failed record logs are kept. Summary counts still cover every processed record. `with_log_limit(Some(n))` changes the cap, `with_log_limit(None)` keeps every log, and `without_logs()` stores none.

Integer arithmetic returns an evaluation error on overflow, division or remainder by zero, and negation of `i64::MIN`. Floating-point arithmetic, including mixed integer and float operations, rejects non-finite results such as NaN and infinity. Finite results, including underflow to zero, stay valid. Literals, field reads, and casts do not use this rule.

## Workspace layout

| Crate | Purpose |
| --- | --- |
| `satva-types` | Records, values, schemas, and data types |
| `satva-expr` | Expression builders and evaluation |
| `satva-parser` | Parsing expression strings |
| `satva-core` | Pipeline orchestration and built-in stages |
| `satva-io` | CSV, TSV, JSONL, JSON array, Parquet, and Excel readers and writers |
| `satva-runner` | YAML load and run, shared by the CLI and Python |
| `satva-cli` | Command-line interface over `satva-runner` |
| `satva-arrow` | Columnar batches and vectorized kernels |
| `satva-execution` | Parallel execution over batches |
| `satva-python` | PyO3 bindings (`satva.run`) |
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

`bash scripts/check.sh` also runs `cargo audit` and a release build. Install `cargo-audit` separately. A workspace build compiles `satva-python`, which needs Python development headers (`python3-dev` or the equivalent for your platform).

## Documentation

- [Getting started and Rust library usage](docs/getting-started.md)
- [Pipeline configuration](docs/pipeline-config.md)
- [Expression language](docs/expression-language.md)
- [Architecture](docs/architecture.md)
