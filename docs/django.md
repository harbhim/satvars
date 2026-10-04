# Django

Pass a Django `FileField` value to `satva.run` as `source`. Satva opens that file object, checks that the stored name is a data file, and runs the YAML pipeline against those bytes.

The YAML file still describes the format and the stages. `source.path` in the YAML is unused for that call. `source.type` must match the file extension.

## Pipeline YAML

```yaml
source:
  type: csv
  path: upload.csv

sink:
  type: json
  path: media/cleaned/employees.jsonl

schema:
  infer: true

stages:
  - type: schema_validation
  - type: filter
    expression: 'active == true && salary >= 70000'
  - type: set_field
    field: bonus
    expression: 'salary * 0.15'
```

`path: upload.csv` is a placeholder. The CLI can still run this file when that path exists on disk. A Django call replaces it with the `FileField` contents.

## Model and call

```python
import satva
from django.db import models


class EmployeeImport(models.Model):
    spreadsheet = models.FileField(upload_to="imports/")


def clean_import(row):
    summary = satva.run(
        "pipelines/employees.yaml",
        source=row.spreadsheet,
    )
    return summary
```

`row.spreadsheet` is a `FieldFile`. Satva calls `open("rb")`, `read()`, and `close()` on it. The storage backend can be local disk or a remote store. A local `.path` is not required.

`stop_on_error=True` still raises `RuntimeError` on the first record failure.

## Which files are accepted

The `FileField` name must end with a data extension, and that extension must agree with `source.type`:

| YAML `source.type` | File name extension |
| --- | --- |
| `csv` | `.csv` |
| `tsv` | `.tsv`, `.tab` |
| `json` | `.jsonl`, `.ndjson`, `.json` |
| `json_array` | `.json` |
| `parquet` | `.parquet` |
| `excel` | `.xlsx`, `.xls`, `.ods` |

`type: json` reads JSONL, including when the name ends in `.json`. `type: json_array` reads one JSON array and requires `.json`.

Other names, such as `photo.png` on an `ImageField`, raise `RuntimeError` and the pipeline does not run. An extension that disagrees with `source.type` (for example `people.jsonl` with `type: csv`) also raises `RuntimeError`.

`source` may also be a `django.core.files.File`, including an uploaded file that has not been saved on a model yet:

```python
summary = satva.run("pipelines/employees.yaml", source=request.FILES["spreadsheet"])
```

The upload's `.name` follows the same extension rules.

## Compare uploaded rows with stored values

The `external` stage compares each uploaded row with values you already have. Those values can be a file, a list of records, or a lookup. Django is one place the lookup can read them from. The stage itself does not know that they came from a model.

```yaml
source:
  type: csv
  path: products.csv

sink:
  type: json
  path: media/cleaned/products.jsonl

stages:
  - type: external
    name: catalog
    key: sku
    compare: [name, price]
    on_missing: continue
    on_match: skip
    on_differ: replace
```

```python
import satva


def catalog(key):
    product = Product.objects.filter(sku=key["sku"]).first()
    if product is None:
        return None
    return {"name": product.name, "price": str(product.price)}


def import_products(upload):
    return satva.run(
        "pipelines/products.yaml",
        source=upload.spreadsheet,
        externals={"catalog": catalog},
    )
```

A matching `name` and `price` is skipped. A different stored product replaces those fields on the row. A sku with no stored product is written as uploaded. To reject a difference instead, set `on_differ: fail`. The reason is in `summary["logs"]`.

Returned values must be `None`, `bool`, `int`, `float`, or `str`. Convert a Django `Decimal` first. CSV cells are strings, so compare them with strings. The lookup runs once per row on the request thread.

The same YAML also accepts `externals={"catalog": [...]}` or, with `path` instead of `name`, a file of product values. See [Pipeline configuration](pipeline-config.md).
