#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::{HashMap, HashSet};

use crate::{
    Annotation, AttrValue, Marker, ParseResult, ParserConfig, RecoveryStrategy, Segment, parse,
};
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

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Annotation(tag='{}', attrs={:?})",
            self.inner.tag, self.inner.attrs
        ))
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

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Segment(text='{}', annotations={})",
            self.inner.text.replace('\'', "\""),
            self.inner
                .annotations
                .iter()
                .map(|a| format!("Annotation(tag='{}')", a.tag))
                .collect::<Vec<_>>()
                .join(", ")
        ))
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

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Marker(pos={}, annotation=Annotation(tag='{}'))",
            self.inner.pos, self.inner.annotation.tag
        ))
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

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "ParseResult(text_len={}, segments={}, markers={})",
            self.inner.text.len(),
            self.inner.segments.len(),
            self.inner.markers.len()
        ))
    }
}

fn default_config() -> ParserConfig {
    let recognized_tags: HashSet<String> = ["cite", "note", "todo", "claim", "risk", "code"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut per_tag_recovery: HashMap<String, RecoveryStrategy> = HashMap::new();
    per_tag_recovery.insert("cite".into(), RecoveryStrategy::RetroLine);
    for tag in ["note", "todo", "claim", "risk", "code"] {
        per_tag_recovery.insert(tag.into(), RecoveryStrategy::ForwardUntilTag);
    }
    ParserConfig {
        recognized_tags,
        per_tag_recovery,
        trim_punctuation: true,
        case_sensitive_tags: false,
        ..ParserConfig::default()
    }
}

#[pyfunction(name = "parse")]
pub fn py_parse(py: Python<'_>, input: &str) -> PyResult<PyObject> {
    let cfg = default_config();
    let result = parse(input, &cfg);
    Py::new(py, PyParseResult { inner: result }).map(|obj| obj.into_py(py))
}

#[pymodule]
#[pyo3(name = "_kindaxml_rs")]
pub fn python_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyParseResult>()?;
    m.add_class::<PySegment>()?;
    m.add_class::<PyAnnotation>()?;
    m.add_class::<PyMarker>()?;
    m.add_function(wrap_pyfunction!(py_parse, m)?)?;
    Ok(())
}
