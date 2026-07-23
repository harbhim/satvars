# satva-rs

A modular data pipeline engine in Rust. Reads records from a source, passes them through a chain of transformation/validation stages, and writes the result to a sink. Supports JSONL and CSV with an expression language for filters and computed fields.

## Quick Start

```bash
cargo build --release
cargo test
```

## CLI Usage

The `satva` CLI runs pipelines defined in YAML config files:

```bash
cargo run -p satva-cli -- run --config crates/satva-cli/examples/pipeline.yaml
```

This processes `employees.jsonl` (20 employee records), applies schema validation, filters for active Engineering employees with salary >= $70k, computes bonus and display_name fields, and writes the result to `cleaned_employees.jsonl`.

### Config Structure

```yaml
source:
  type: json          # or: csv
  path: input.jsonl

sink:
  type: json          # or: csv
  path: output.jsonl

schema:
  infer: true
  sample_size: 1000

stages:
  - type: schema_validation
  - type: filter
    expression: "is_not_null(salary) && active == true && salary >= 70000"
  - type: rename_field
    from: old_name
    to: new_name
  - type: set_field
    field: bonus
    expression: "salary * 0.15"
  - type: select_fields
    fields: [id, name, bonus]
  - type: remove_field
    fields: [temp, debug]
```

Available stages: `schema_validation`, `filter`, `rename_field`, `select_fields`, `remove_field`, `set_field`.

## Using as a Library

```rust
use satva_core::{
    FilterStage, Pipeline, PipelineBuilder,
    SelectFieldsStage, SetFieldStage,
};
use satva_expr::{field, lit};
use satva_io::source::JsonSource;
use satva_io::sink::JsonSink;

let mut pipeline = Pipeline::new(Box::new(JsonSource::new("input.jsonl")));
pipeline.add_stage(Box::new(FilterStage::new(
    field("active").equal_to(lit(true))
        .and(field("salary").greater_than(lit(70_000.0)))
)));
pipeline.add_stage(Box::new(SetFieldStage::new(
    "bonus",
    field("salary").times(lit(0.15)),
)));
pipeline.set_sink(Box::new(JsonSink::new("output.jsonl")));

let result = pipeline.run(satva_core::PipelineOptions::new())?;
println!("{:#?}", result.summary);
```

## Benchmarks

```bash
cargo bench -p satva-benchmarks
```

| Benchmark | Time |
|---|---|
| Parse a field name | ~386 ns |
| Parse a complex filter | ~3.73 µs |
| Evaluate a field lookup | ~27 ns |
| Evaluate a complex filter | ~271 ns |
| Pipeline (filter, 1000 records) | ~1.76 ms |
| Full pipeline (filter + transforms, 1000 records) | ~2.43 ms |

HTML reports are generated in `benchmarks/target/criterion/`.

## Project Structure

| Crate | Description |
|---|---|
| `satva-types` | Core types: Record, Value, Schema, DataType |
| `satva-expr` | Expression tree + evaluator |
| `satva-parser` | String-to-Expression parser |
| `satva-core` | Pipeline orchestration and built-in stages |
| `satva-io` | Source/sink implementations (CSV, JSONL) |
| `satva-cli` | CLI binary driven by YAML config |
| `satva-execution` | (stub) Future parallel execution |
| `satva-arrow` | (stub) Future Apache Arrow interop |
| `satva-python` | (stub) Future PyO3 bindings |

## Documentation

- [Architecture](docs/architecture.md)
- [Getting Started](docs/getting-started.md)
- [Pipeline Configuration](docs/pipeline-config.md)
- [Expression Language](docs/expression-language.md)
