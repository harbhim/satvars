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
