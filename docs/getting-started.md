# Getting Started

## Prerequisites

- Rust 2024 edition (MSRV 1.85+)
- Cargo

## Build

```bash
cargo build --release
```

## Test

```bash
cargo test
```

## Run the Example Pipeline

```bash
cargo run -p satva-cli -- run --config crates/satva-cli/examples/pipeline.yaml
```

This processes `employees.jsonl` (20 employee records), applies schema validation, filters for active Engineering employees with salary >= $70k, computes bonus and display_name fields, and writes the result to `cleaned_employees.jsonl`.

## Run Benchmarks

```bash
cargo bench -p satva-benchmarks
```

Results include:

| Benchmark | Time |
|---|---|
| Parse a field name | ~386 ns |
| Parse a complex filter | ~3.73 µs |
| Evaluate a field lookup | ~27 ns |
| Evaluate a complex filter | ~271 ns |
| Pipeline (filter, 1000 records) | ~1.76 ms |
| Full pipeline (filter + transforms, 1000 records) | ~2.43 ms |

HTML reports are generated in `benchmarks/target/criterion/`.

## Using the Library

### Builder API (Rust)

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

### YAML Config

See `crates/satva-cli/examples/pipeline.yaml` for a complete example.
