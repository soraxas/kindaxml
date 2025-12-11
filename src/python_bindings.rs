#![allow(unsafe_op_in_unsafe_fn)]

use crate::{
    Annotation, AttrValue, Marker, ParseResult, ParserConfig, RecoveryStrategy, Segment,
    UnknownMode, parse,
};
use pyo3::prelude::*;
use pyo3::types::PyType;

#[pyclass(name = "Annotation")]
#[derive(Clone)]
pub struct PyAnnotation {
    inner: Annotation,
}

#[pymethods]
impl PyAnnotation {
    #[classattr]
    const __doc__: &'static str =
        "Annotation(tag: str, attrs: dict[str, bool | str]) -> annotation attached to a span.";

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
    #[classattr]
    const __doc__: &'static str = "Segment(text: str, annotations: list[Annotation]).";

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
    #[classattr]
    const __doc__: &'static str =
        "Marker(pos: int, annotation: Annotation) from self-closing tags.";

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
    #[classattr]
    const __doc__: &'static str =
        "ParseResult(text: str, segments: list[Segment], markers: list[Marker]).";

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

#[pyclass(name = "ParserConfig")]
pub struct PyParserConfig {
    inner: ParserConfig,
}

#[pymethods]
impl PyParserConfig {
    #[classattr]
    const __doc__: &'static str = "ParserConfig() -> mutable parser configuration.";

    #[new]
    pub fn new() -> Self {
        Self {
            inner: ParserConfig::default_llm_friendly_config(),
        }
    }

    #[classmethod]
    pub fn default_llm_friendly_config(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: ParserConfig::default_llm_friendly_config(),
        }
    }

    #[classmethod]
    pub fn default_cite_config(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: ParserConfig::default_cite_config(),
        }
    }

    /// Replace the recognized tag set.
    pub fn set_recognized_tags(&mut self, tags: Vec<String>) {
        self.inner.recognized_tags = tags.into_iter().collect();
    }

    /// Set the unknown tag handling mode: "strip", "passthrough", or "treat_as_text".
    pub fn set_unknown_mode(&mut self, mode: &str) -> PyResult<()> {
        self.inner.unknown_mode = match mode.to_ascii_lowercase().as_str() {
            "strip" => UnknownMode::Strip,
            "passthrough" => UnknownMode::Passthrough,
            "treat_as_text" => UnknownMode::TreatAsText,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown mode '{}'",
                    other
                )));
            }
        };
        Ok(())
    }

    /// Set recovery strategy for a specific tag: "retro_line", "forward_until_tag", "forward_until_newline", "forward_next_token", or "noop".
    pub fn set_recovery_strategy(&mut self, tag: &str, strategy: &str) -> PyResult<()> {
        let strat = match strategy.to_ascii_lowercase().as_str() {
            "retro_line" => RecoveryStrategy::RetroLine,
            "forward_until_tag" => RecoveryStrategy::ForwardUntilTag,
            "forward_until_newline" => RecoveryStrategy::ForwardUntilNewline,
            "forward_next_token" => RecoveryStrategy::ForwardNextToken,
            "noop" => RecoveryStrategy::Noop,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown recovery strategy '{}'",
                    other
                )));
            }
        };
        self.inner.per_tag_recovery.insert(tag.to_string(), strat);
        Ok(())
    }

    /// Toggle punctuation trimming for retro spans.
    pub fn set_trim_punctuation(&mut self, val: bool) {
        self.inner.trim_punctuation = val;
    }

    /// Toggle auto-close behavior when encountering any new tag.
    pub fn set_autoclose_on_any_tag(&mut self, val: bool) {
        self.inner.autoclose_on_any_tag = val;
    }

    /// Toggle auto-close behavior when seeing the same tag again.
    pub fn set_autoclose_on_same_tag(&mut self, val: bool) {
        self.inner.autoclose_on_same_tag = val;
    }

    /// Toggle case-sensitive tag matching.
    pub fn set_case_sensitive_tags(&mut self, val: bool) {
        self.inner.case_sensitive_tags = val;
    }
}

#[pyfunction(name = "parse")]
#[pyo3(text_signature = "(text, config=None)")]
/// Parse KindaXML text using the default config (case-insensitive tags, cite retro, others forward).
pub fn py_parse(
    py: Python<'_>,
    input: &str,
    config: Option<&PyParserConfig>,
) -> PyResult<PyObject> {
    let cfg = config
        .map(|c| c.inner.clone())
        .unwrap_or_else(ParserConfig::default_llm_friendly_config);
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
    m.add_class::<PyParserConfig>()?;
    m.add_function(wrap_pyfunction!(py_parse, m)?)?;
    Ok(())
}
