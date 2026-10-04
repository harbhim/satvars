# Pipeline Configuration

Pipeline configs are YAML files that define a source, optional sink, schema inference settings, and a sequence of processing stages.

## Structure

```yaml
source:
  type: json          # json, json_array, csv, tsv, parquet, excel
  path: input.jsonl

sink:
  type: json          # json, json_array, csv, tsv, parquet, excel
  path: output.jsonl

schema:
  infer: true         # auto-detect field types from a sample
  sample_size: 1000   # rows to sample (default: 1000)

stages:
  - type: schema_validation
  - type: filter
    expression: "age >= 18"
  - type: rename_field
    from: old_name
    to: new_name
  - type: select_fields
    fields: [id, name, email]
  - type: remove_field
    fields: [temp, debug]
  - type: set_field
    field: bonus
    expression: "salary * 0.15"
```

Django can supply this source from a `FileField` file object. The YAML `source.type` still selects the format. See [Django](django.md).

## Source

### JSON Source

Reads JSONL (one JSON object per line). JSON types are mapped to satva types as follows:

| JSON Type | satva Type |
|---|---|
| number (integer) | Int64 |
| number (float) | Float64 |
| string | String |
| boolean | Boolean |
| null | Null |
| array / object | String (serialized) |

### JSON Array Source

Reads one JSON file whose top-level value is an array of objects. Scalar mapping matches the JSONL source.

### CSV Source

Reads CSV with a header row. **All values are read as strings.** Type coercion happens later if a `schema_validation` stage is configured.

### TSV Source

Reads tab-separated values with a header row. Cell values are strings, same as CSV.

### Parquet Source

Reads a Parquet file. Integers become `Int64`, floats become `Float64`, booleans stay booleans, and strings stay strings. Dates, timestamps, and dictionary-encoded columns are read as strings.

### Excel Source

Reads `.xlsx`, `.xls`, or `.ods`. The first row is the header. Typed cells become integers, floats, booleans, or strings. Dates are strings. Empty cells are null.

```yaml
source:
  type: excel
  path: employees.xlsx
  sheet: People   # optional; defaults to the first worksheet
```

## Sink

### JSON Sink

Writes JSONL. Field order follows insertion order (preserved from source or stage reordering).

### JSON Array Sink

Writes one JSON array. `finish` closes the array, and a run that writes nothing still produces `[]`.

### CSV Sink

Writes CSV with a header row. Column order follows the first record's field order.

### TSV Sink

Writes tab-separated values. Column order follows the first record, same as CSV.

### Parquet Sink

Writes an uncompressed Parquet file. Rows are buffered until `finish`. Columns come from the first record. A later float widens an integer column. Missing fields are null, and extra fields are dropped.

### Excel Sink

Writes an `.xlsx` workbook, buffered until `finish`. Columns come from the first record. Missing fields are blank, and extra fields are dropped. Integers and floats are Excel numbers.

```yaml
sink:
  type: excel
  path: cleaned.xlsx
  sheet: People   # optional; defaults to Sheet1
```

## Schema

```yaml
schema:
  infer: true        # required for schema_validation stage
  sample_size: 1000  # optional, default 1000
```

Schema inference samples records from the source and uses majority-vote type detection. Results are printed at pipeline startup.

## Stages

### schema_validation

Coerces record fields to match the inferred schema types. Handles:
- Type coercion from strings to int/float/bool
- Missing fields (error if non-nullable)
- Nullable field acceptance
- Empty string rejection on non-nullable fields

### filter

Keeps only records where the expression evaluates to `true`. Records evaluating to `false` are skipped. Uses short-circuit evaluation for null safety.

```yaml
- type: filter
  expression: "is_not_null(salary) && active == true && salary >= 70000"
```

### rename_field

Renames a single field.

```yaml
- type: rename_field
  from: education
  to: edu
```

### select_fields

Keeps only the specified fields (drops all others).

```yaml
- type: select_fields
  fields: [employee_id, first_name, department, salary]
```

### remove_field

Removes specified fields (keeps all others).

```yaml
- type: remove_field
  fields: [temp_data, internal_id]
```

### set_field

Adds or overwrites a field with the result of an expression.

```yaml
- type: set_field
  field: display_name
  expression: "upper(first_name) + \" \" + upper(last_name)"
```

String literals inside expressions must use escaped quotes in YAML: `"\"value\""`.

### external

Compares each row with values that come from outside the source file. The values can be another file, or any records the caller supplies. The stage then continues, skips, fails, or replaces fields. The comparison is the same wherever the values came from.

```yaml
- type: external
  key: sku
  path: catalog.jsonl
  compare: [name, price]
  on_missing: continue   # no external row for this key
  on_match: skip         # compared fields are equal
  on_differ: replace     # copy those fields from the external values
```

`key` is one field name, or a list of field names. A row with a missing or null key fails. `compare` lists the fields to compare. When it is omitted, every external field except the key is compared. `compare: []` checks only that the key exists.

`on_missing` defaults to `continue`. `on_match` defaults to `skip`. `on_differ` defaults to `replace`. Each one is `continue`, `skip`, `fail`, or `replace`. `correct` is another name for `replace`. `on_missing: replace` is rejected, because there is no external value to copy. `skip` and `fail` are written to the run log. Two external rows with the same key are an error before any source row is read.

`path` reads those values with the same readers as a source. `format` is `json`, `json_array`, `csv`, `tsv`, `parquet`, or `excel`. It is inferred from the extension. A `.json` file needs `format`, because that extension is used for both JSONL and a JSON array. `sheet` selects an Excel worksheet.

To pass the values from Rust or Python instead of a file, use `name` and omit `path`. A stage cannot set both.

```yaml
- type: external
  name: catalog
  key: sku
  compare: [name, price]
```

```rust,ignore
use std::collections::HashMap;
use satva_core::ExternalData;

let mut externals = HashMap::new();
externals.insert("catalog".to_string(), ExternalData::Records(existing_products));
satva_runner::run_config_with(config, options, &externals)?;
```

From Python, `externals` is a dict. A list of dicts is indexed by `key`. A dict of dicts maps one key value to the fields to compare. A callable receives the key fields and returns those fields, or `None` when the key has no external value. Field values are `None`, `bool`, `int`, `float`, or `str`. An integer and the string `"1"` are different values.

```python
satva.run("pipelines/products.yaml", externals={
    "catalog": [
        {"sku": "A1", "name": "Widget", "price": "10"},
    ]
})
```

```python
def catalog(key):
    current = lookup(key["sku"])
    if current is None:
        return None
    return {"name": current.name, "price": current.price}

satva.run("pipelines/products.yaml", externals={"catalog": catalog})
```

## Full Example

```yaml
source:
  type: json
  path: employees.jsonl

sink:
  type: json
  path: cleaned_employees.jsonl

schema:
  infer: true
  sample_size: 1000

stages:
  - type: schema_validation
  - type: filter
    expression: "is_not_null(salary) && is_not_null(active) && active == true && department == \"Engineering\" && salary >= 70000"
  - type: set_field
    field: bonus
    expression: "salary * 0.15"
  - type: set_field
    field: display_name
    expression: "upper(first_name) + \" \" + upper(trim(last_name))"
  - type: select_fields
    fields: [employee_id, display_name, department, salary, bonus, rating]
```
