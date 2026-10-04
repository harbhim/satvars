use std::collections::HashMap;
use std::sync::Arc;

use pyo3::exceptions::PyRuntimeError;
use pyo3::ffi::{self, Py_INCREF};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList};
use satva_core::{ExternalData, ExternalValues};
use satva_types::{Record, Value};

/// Asks a Python callable for the external values of one key.
struct PyLookup {
    name: String,
    callback: Py<PyAny>,
}

impl ExternalValues for PyLookup {
    fn get(&self, key: &Record) -> Result<Option<Record>, String> {
        Python::attach(|py| {
            self.lookup(py, key)
                .map_err(|err| exception_message(py, err))
        })
    }
}

impl PyLookup {
    fn lookup(&self, py: Python<'_>, key: &Record) -> PyResult<Option<Record>> {
        let key = record_to_py(py, key)?;
        let result = self.callback.bind(py).call1((key,)).map_err(|err| {
            PyRuntimeError::new_err(format!("{}: {}", self.name, exception_message(py, err)))
        })?;
        if result.is_none() {
            return Ok(None);
        }
        let dict = result.downcast::<PyDict>().map_err(|_| {
            PyRuntimeError::new_err(
                "lookup must return the external values as a dict, or None when there is no value",
            )
        })?;
        Ok(Some(record_from_dict(&self.name, dict)?))
    }
}

pub(crate) fn data_from_py(
    externals: Option<&Bound<'_, PyAny>>,
) -> PyResult<HashMap<String, ExternalData>> {
    let Some(externals) = externals else {
        return Ok(HashMap::new());
    };
    let dict = externals.downcast::<PyDict>().map_err(|_| {
        PyRuntimeError::new_err(
            "externals must be a dict of stage name to a list of records, a key-to-values mapping, or a lookup callable",
        )
    })?;

    let mut provided = HashMap::new();
    for (key, value) in dict.iter() {
        let name: String = key.extract().map_err(|_| {
            PyRuntimeError::new_err("externals keys must be strings matching the YAML stage name")
        })?;
        if name.is_empty() {
            return Err(PyRuntimeError::new_err("externals keys must not be empty"));
        }
        provided.insert(name.clone(), external_data(&name, &value)?);
    }
    Ok(provided)
}

fn external_data(name: &str, value: &Bound<'_, PyAny>) -> PyResult<ExternalData> {
    if value.is_callable() {
        return Ok(ExternalData::Lookup(Arc::new(PyLookup {
            name: name.to_string(),
            callback: value.clone().unbind(),
        })));
    }
    if let Ok(list) = value.downcast::<PyList>() {
        let mut records = Vec::new();
        for (index, item) in list.iter().enumerate() {
            let dict = item.downcast::<PyDict>().map_err(|_| {
                PyRuntimeError::new_err(format!(
                    "externals['{name}'][{index}] must be a dict of field names to values"
                ))
            })?;
            records.push(record_from_dict(name, dict)?);
        }
        return Ok(ExternalData::Records(records));
    }
    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut entries = Vec::new();
        for (key, item) in dict.iter() {
            let key_value = py_to_value("key", &key)?;
            let fields = item.downcast::<PyDict>().map_err(|_| {
                PyRuntimeError::new_err(format!(
                    "externals['{name}'] values must be dicts of field names to values"
                ))
            })?;
            entries.push((key_value, record_from_dict(name, fields)?));
        }
        return Ok(ExternalData::Mapping(entries));
    }
    Err(PyRuntimeError::new_err(format!(
        "externals['{name}'] must be a list of records, a key-to-values mapping, or a lookup callable"
    )))
}

fn record_from_dict(name: &str, fields: &Bound<'_, PyDict>) -> PyResult<Record> {
    let mut record = Record::new();
    for (key, value) in fields.iter() {
        let key: String = key.extract().map_err(|_| {
            PyRuntimeError::new_err(format!("externals['{name}'] field names must be strings"))
        })?;
        record.insert(&key, py_to_value(&key, &value)?);
    }
    Ok(record)
}

fn record_to_py<'py>(py: Python<'py>, record: &Record) -> PyResult<Bound<'py, PyDict>> {
    let row = PyDict::new(py);
    for (field, value) in record {
        row.set_item(field, value_to_py(py, value)?)?;
    }
    Ok(row)
}

fn value_to_py<'py>(py: Python<'py>, value: &Value) -> PyResult<Bound<'py, PyAny>> {
    match value {
        Value::Null => Ok(py.None().into_bound(py)),
        Value::Int64(value) => Ok((*value).into_pyobject(py)?.into_any()),
        Value::Float64(value) => Ok((*value).into_pyobject(py)?.into_any()),
        Value::Boolean(value) => Ok(owned_bool(py, *value)),
        Value::String(value) => Ok(value.into_pyobject(py)?.into_any()),
    }
}

/// Own a reference to Python's `True` or `False` singleton.
///
/// `PyBool::new` only borrows that singleton. Cloning the borrow moves a
/// `Bound<PyBool>` out of a `Borrowed`, which is rejected.
fn owned_bool(py: Python<'_>, value: bool) -> Bound<'_, PyAny> {
    // SAFETY: `Py_True` and `Py_False` are immortal singletons and never null.
    // `Py_INCREF` turns that borrowed pointer into an owned reference, which
    // `Bound::from_owned_ptr` requires.
    unsafe {
        let ptr = if value { ffi::Py_True() } else { ffi::Py_False() };
        Py_INCREF(ptr);
        Bound::from_owned_ptr(py, ptr)
    }
}

fn py_to_value(field: &str, value: &Bound<'_, PyAny>) -> PyResult<Value> {
    if value.is_none() {
        return Ok(Value::Null);
    }
    // bool is a subclass of int, so it has to be checked first.
    if value.is_instance_of::<PyBool>() {
        return Ok(Value::Boolean(value.extract::<bool>()?));
    }
    if value.is_instance_of::<PyInt>() {
        let number = value.extract::<i64>().map_err(|_| {
            PyRuntimeError::new_err(format!("field '{field}' integer does not fit in i64"))
        })?;
        return Ok(Value::Int64(number));
    }
    if value.is_instance_of::<PyFloat>() {
        let number = value.extract::<f64>()?;
        if !number.is_finite() {
            return Err(PyRuntimeError::new_err(format!(
                "field '{field}' must be a finite float"
            )));
        }
        return Ok(Value::Float64(number));
    }
    if let Ok(text) = value.extract::<String>() {
        return Ok(Value::String(text));
    }
    Err(PyRuntimeError::new_err(format!(
        "field '{field}' must be null, bool, int, float, or str"
    )))
}

fn exception_message(py: Python<'_>, err: PyErr) -> String {
    let value = err.value(py);
    value
        .str()
        .map(|text| text.to_string())
        .unwrap_or_else(|_| err.to_string())
}
