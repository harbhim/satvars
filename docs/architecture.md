# Architecture

satva-rs is a modular data pipeline engine built in Rust. It reads records from a source, passes them through a chain of transformation/validation stages, and writes the result to a sink.

## Crate Overview

```
satva-types          — Core types: Record, Value, Schema, Field, DataType
satva-expr           — Expression tree + evaluator (field > 18, upper(name), etc.)
satva-parser         — String-to-Expression parser ("salary >= 50000")
satva-core           — Pipeline orchestration, stage trait, built-in stages
satva-io             — Source/sink implementations (CSV, JSONL)
satva-cli            — CLI binary driven by YAML config
satva-execution      — (stub) Future parallel execution engine
satva-arrow          — (stub) Future Apache Arrow interop
satva-python         — (stub) Future PyO3 bindings
```

## Dependency Flow

```
satva-types (no deps on other satva crates)
  ^--- satva-expr
  ^--- satva-parser
  ^--- satva-core ----^--- satva-io
                   ^--- satva-cli
```

## Core Concepts

### Record

A `Record` is a collection of named fields backed by an `IndexMap<String, Value>`. Field ordering is preserved — insertion order determines output order for sinks.

### Value

Values are a strict enum:

```rust
pub enum Value {
    Null,
    Int64(i64),
    Float64(f64),
    Boolean(bool),
    String(String),
}
```

Comparisons (`>`, `>=`, `<`, `<=`) between different numeric types (Int64 vs Float64) are handled automatically. Comparisons involving `Null` return `false`.

### Pipeline

A `Pipeline` connects a `Source`, zero or more `PipelineStage`s, and optionally a `Sink`. Stages execute sequentially per-record. Each stage returns:

| Outcome | Meaning |
|---|---|
| `Continue` | Record proceeds to the next stage |
| `Skip` | Record is skipped (logged, not written to sink) |
| `Fail` | Record fails (logged as error, not written to sink) |

Short-circuit evaluation protects `&&` and `||` — if the left side determines the result, the right side is never evaluated. This allows patterns like `is_not_null(salary) && salary > 50000` to work safely with null fields.

### Expressions

Expressions are immutable AST nodes built with the builder API in `satva-expr` or parsed from strings via `satva-parser`. The evaluator resolves them against a Record at runtime.
