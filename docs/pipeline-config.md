# Pipeline Configuration

Pipeline configs are YAML files that define a source, optional sink, schema inference settings, and a sequence of processing stages.

## Structure

```yaml
source:
  type: json          # or: csv
  path: input.jsonl

sink:
  type: json          # or: csv
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

### CSV Source

Reads CSV with a header row. **All values are read as strings.** Type coercion happens later if a `schema_validation` stage is configured.

## Sink

### JSON Sink

Writes JSONL. Field order follows insertion order (preserved from source or stage reordering).

### CSV Sink

Writes CSV with a header row. Column order follows the first record's field order.

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
