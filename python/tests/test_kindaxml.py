from __future__ import annotations

import pathlib
import sys

import pytest

PROJECT_ROOT = pathlib.Path(__file__).resolve().parents[2]
PYTHON_SRC = PROJECT_ROOT / "python"
sys.path.insert(0, str(PYTHON_SRC))

try:
    from kindaxml import PyAnnotation, PyParseResult, PySegment, parse
except ImportError:  # pragma: no cover - handled by pytest skip
    pytest.skip(
        "Native extension not built. Run `maturin develop` or `maturin build` first.",
        allow_module_level=True,
    )


def test_parse_returns_typed_result() -> None:
    res = parse("We shipped <cite id=1>last week</cite>.")
    assert isinstance(res, PyParseResult)
    assert res.text == "We shipped last week."
    assert any(isinstance(seg, PySegment) for seg in res.segments)
    cite_ann = res.segments[1].annotations[0]
    assert isinstance(cite_ann, PyAnnotation)
    assert cite_ann.tag == "cite"
    assert cite_ann.attrs["id"] == "1"


def test_parse_markers() -> None:
    res = parse("Todo <todo id=3/>now")
    assert res.markers[0].annotation.tag == "todo"
    assert res.markers[0].annotation.attrs["id"] == "3"
