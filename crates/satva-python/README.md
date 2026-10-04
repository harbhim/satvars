# satva

[Satva](https://github.com/harbhim/satvars) is a data pipeline engine. This package runs the same YAML pipelines as the `satva` command-line tool.

Requires Python 3.9 or newer. Licensed under the MIT License.

## Install

```bash
pip install satva
```

## Usage

```python
import satva

summary = satva.run("pipeline.yaml")
print(summary["processed"], summary["succeeded"], summary["skipped"], summary["failed"])

# Raises RuntimeError on the first record failure.
satva.run("pipeline.yaml", stop_on_error=True)
```

`satva.run` returns a dict with `processed`, `succeeded`, `skipped`, `failed`, and `logs`. `logs` is a list of strings. `stop_on_error` defaults to `False`: failed records are counted and the call returns. File paths inside the YAML config are relative to the process working directory.

## Django

Pass a model `FileField` value as `source`. Satva opens that file object and reads its bytes. The file name must be a data file, and the extension must match `source.type` in the YAML. `source.path` in the YAML is not used for this call.

```python
summary = satva.run("pipelines/employees.yaml", source=row.spreadsheet)
```

```yaml
source:
  type: csv
  path: upload.csv

sink:
  type: json
  path: media/cleaned/employees.jsonl

stages:
  - type: filter
    expression: 'active == true && salary >= 70000'
```

| YAML `source.type` | Accepted file name |
| --- | --- |
| `csv` | `.csv` |
| `tsv` | `.tsv`, `.tab` |
| `json` | `.jsonl`, `.ndjson`, `.json` |
| `json_array` | `.json` |
| `parquet` | `.parquet` |
| `excel` | `.xlsx`, `.xls`, `.ods` |

A non-data name such as `photo.png` raises `RuntimeError`. An uploaded `django.core.files.File` is accepted the same way, using its `.name`. Full steps are in the [Django guide](https://github.com/harbhim/satvars/blob/master/docs/django.md).

## External values

An `external` stage compares each row with values from outside the file. Point `path` at another data file, or pass the values to `satva.run` under the stage `name`. The YAML `on_match`, `on_differ`, and `on_missing` settings choose continue, skip, fail, or replace. The same comparison runs for a file, a list, or a lookup.

```yaml
stages:
  - type: external
    name: catalog
    key: sku
    compare: [name, price]
    on_match: skip
    on_differ: replace
```

```python
summary = satva.run(
    "pipelines/products.yaml",
    externals={
        "catalog": [
            {"sku": "A1", "name": "Widget", "price": "10"},
        ]
    },
)
```

A callable can supply one key at a time. It receives the key fields and returns the values to compare, or `None` when that key has no external value.

```python
def catalog(key):
    current = lookup(key["sku"])
    if current is None:
        return None
    return {"name": current.name, "price": str(current.price)}

summary = satva.run("pipelines/products.yaml", externals={"catalog": catalog})
```

Field values are `None`, `bool`, `int`, `float`, or `str`. A named stage with no values raises `RuntimeError` before any row is read. A stage with `path` reads that file and does not use `externals`.

```yaml
source:
  type: json
  path: input.jsonl

sink:
  type: json
  path: output.jsonl

stages:
  - type: filter
    expression: 'active == true && salary >= 70000'
  - type: set_field
    field: bonus
    expression: 'salary * 0.15'
```

Sources and sinks: CSV, TSV, JSONL (`json`), JSON array (`json_array`), Parquet, and Excel. Pipeline and expression details are in the [repository docs](https://github.com/harbhim/satvars/tree/master/docs).
