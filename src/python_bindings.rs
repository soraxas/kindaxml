#![allow(unsafe_op_in_unsafe_fn)]

use crate::{Annotation, AttrValue, Marker, ParseResult, ParserConfig, Segment, parse};
use pyo3::prelude::*;

#[pyclass(name = "Annotation")]
#[derive(Clone)]
pub struct PyAnnotation {
    inner: Annotation,
}

#[pymethods]
impl PyAnnotation {
    #[getter]
    fn tag(&self) -> &str {
        &self.inner.tag
    }

    #[getter]
    fn attrs<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new_bound(py);
        for (k, v) in &self.inner.attrs {
            match v {
                AttrValue::Bool(b) => dict.set_item(k, *b)?,
                AttrValue::Str(s) => dict.set_item(k, s)?,
            }
        }
        Ok(dict.into_py(py))
    }
}

#[pyclass(name = "Segment")]
#[derive(Clone)]
pub struct PySegment {
    inner: Segment,
}

#[pymethods]
impl PySegment {
    #[getter]
    fn text(&self) -> &str {
        &self.inner.text
    }

    #[getter]
    fn annotations<'py>(&self, py: Python<'py>) -> PyResult<Vec<Py<PyAnnotation>>> {
        self.inner
            .annotations
            .iter()
            .cloned()
            .map(|a| Py::new(py, PyAnnotation { inner: a }))
            .collect()
    }
}

#[pyclass(name = "Marker")]
#[derive(Clone)]
pub struct PyMarker {
    inner: Marker,
}

#[pymethods]
impl PyMarker {
    #[getter]
    fn pos(&self) -> usize {
        self.inner.pos
    }

    #[getter]
    fn annotation<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAnnotation>> {
        Py::new(
            py,
            PyAnnotation {
                inner: self.inner.annotation.clone(),
            },
        )
    }
}

#[pyclass(name = "ParseResult")]
pub struct PyParseResult {
    inner: ParseResult,
}

#[pymethods]
impl PyParseResult {
    #[getter]
    fn text(&self) -> &str {
        &self.inner.text
    }

    #[getter]
    fn segments<'py>(&self, py: Python<'py>) -> PyResult<Vec<Py<PySegment>>> {
        self.inner
            .segments
            .iter()
            .cloned()
            .map(|s| Py::new(py, PySegment { inner: s }))
            .collect()
    }

    #[getter]
    fn markers<'py>(&self, py: Python<'py>) -> PyResult<Vec<Py<PyMarker>>> {
        self.inner
            .markers
            .iter()
            .cloned()
            .map(|m| Py::new(py, PyMarker { inner: m }))
            .collect()
    }
}

#[pyfunction(name = "parse")]
pub fn py_parse(py: Python<'_>, input: &str) -> PyResult<PyObject> {
    let cfg = ParserConfig::default();
    let result = parse(input, &cfg);
    Py::new(py, PyParseResult { inner: result }).map(|obj| obj.into_py(py))
}

#[pymodule]
#[pyo3(name = "_lib_name")]
pub fn python_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyParseResult>()?;
    m.add_class::<PySegment>()?;
    m.add_class::<PyAnnotation>()?;
    m.add_class::<PyMarker>()?;
    m.add_function(wrap_pyfunction!(py_parse, m)?)?;
    Ok(())
}
