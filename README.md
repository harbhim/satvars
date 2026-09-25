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
