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
            inner: ParserConfig::default(),
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
    pub fn with_recognized_tags<'a>(
        mut slf: PyRefMut<'a, Self>,
        tags: Vec<String>,
    ) -> PyRefMut<'a, Self> {
        slf.inner.recognized_tags = tags.into_iter().collect();
        slf
    }

    /// Set the unknown tag handling mode: "strip", "passthrough", or "treat_as_text".
    pub fn with_unknown_mode<'a>(
        mut slf: PyRefMut<'a, Self>,
        mode: &str,
    ) -> PyResult<PyRefMut<'a, Self>> {
        slf.inner.unknown_mode = match mode.to_ascii_lowercase().as_str() {
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
        Ok(slf)
    }

    /// Set recovery strategy for a specific tag: "retro_line", "forward_until_tag", "forward_until_newline", "forward_next_token", or "noop".
    pub fn with_recovery_strategy<'a>(
        mut slf: PyRefMut<'a, Self>,
        tag: &str,
        strategy: &str,
    ) -> PyResult<PyRefMut<'a, Self>> {
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
        slf.inner.per_tag_recovery.insert(tag.to_string(), strat);
        // if the tag is not recognized yet, add it
        slf.inner.recognized_tags.insert(tag.to_string());
        Ok(slf)
    }

    /// Toggle punctuation trimming for retro spans.
    pub fn with_trim_punctuation<'a>(mut slf: PyRefMut<'a, Self>, val: bool) -> PyRefMut<'a, Self> {
        slf.inner.trim_punctuation = val;
        slf
    }

    /// Toggle auto-close behavior when encountering any new tag.
    pub fn with_autoclose_on_any_tag<'a>(
        mut slf: PyRefMut<'a, Self>,
        val: bool,
    ) -> PyRefMut<'a, Self> {
        slf.inner.autoclose_on_any_tag = val;
        slf
    }

    /// Toggle auto-close behavior when seeing the same tag again.
    pub fn with_autoclose_on_same_tag<'a>(
        mut slf: PyRefMut<'a, Self>,
        val: bool,
    ) -> PyRefMut<'a, Self> {
        slf.inner.autoclose_on_same_tag = val;
        slf
    }

    /// Toggle case-sensitive tag matching.
    pub fn with_case_sensitive_tags<'a>(
        mut slf: PyRefMut<'a, Self>,
        val: bool,
    ) -> PyRefMut<'a, Self> {
        slf.inner.case_sensitive_tags = val;
        slf
    }

    fn __repr__(&self) -> String {
        // format recognized tags as a sorted list
        let mut tags: Vec<_> = self.inner.recognized_tags.iter().cloned().collect();
        tags.sort();
        let tags_s = format!(
            "[{}]",
            tags.iter()
                .map(|t| format!("'{}'", t))
                .collect::<Vec<_>>()
                .join(", ")
        );

        // format unknown mode
        let unknown_mode_s = match self.inner.unknown_mode {
            UnknownMode::Strip => "strip",
            UnknownMode::Passthrough => "passthrough",
            UnknownMode::TreatAsText => "treat_as_text",
        };

        // format per-tag recovery map as sorted entries
        let mut per: Vec<_> = self
            .inner
            .per_tag_recovery
            .iter()
            .map(|(k, v)| {
                let s = match v {
                    RecoveryStrategy::RetroLine => "retro_line",
                    RecoveryStrategy::ForwardUntilTag => "forward_until_tag",
                    RecoveryStrategy::ForwardUntilNewline => "forward_until_newline",
                    RecoveryStrategy::ForwardNextToken => "forward_next_token",
                    RecoveryStrategy::Noop => "noop",
                };
                format!("'{}': {}", k, s)
            })
            .collect();
        per.sort();
        let per_s = format!("{{{}}}", per.join(", "));

        format!(
            "ParserConfig(
   recognized_tags={},
   unknown_mode='{}',
   per_tag_recovery={},
   trim_punctuation={},
   autoclose_on_any_tag={},
   autoclose_on_same_tag={},
   case_sensitive_tags={}
)",
            tags_s,
            unknown_mode_s,
            per_s,
            self.inner.trim_punctuation,
            self.inner.autoclose_on_any_tag,
            self.inner.autoclose_on_same_tag,
            self.inner.case_sensitive_tags
        )
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
