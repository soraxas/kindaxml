from __future__ import annotations

import pathlib
import sys

PROJECT_ROOT = pathlib.Path(__file__).resolve().parents[2]
PYTHON_SRC = PROJECT_ROOT / "python"
sys.path.insert(0, str(PYTHON_SRC))


from kindaxml import (  # noqa: E402
    Annotation,
    Marker,
    ParserConfig,
    ParseResult,
    Segment,
    parse,
)


def test_parse_returns_typed_result() -> None:
    res = parse("We shipped <cite id=1>last week</cite>.")
    assert isinstance(res, ParseResult)
    assert res.text == "We shipped last week."
    assert any(isinstance(seg, Segment) for seg in res.segments)
    cite_ann = res.segments[1].annotations[0]
    assert isinstance(cite_ann, Annotation)
    assert cite_ann.tag == "cite"
    assert cite_ann.attrs["id"] == "1"
    assert "ParseResult" in repr(res)
    assert "Annotation" in repr(cite_ann)


def test_parse_markers() -> None:
    res = parse("Todo <todo id=3/>now")
    assert isinstance(res.markers[0], Marker)
    assert res.markers[0].annotation.tag == "todo"
    assert res.markers[0].annotation.attrs["id"] == "3"
    assert "Marker" in repr(res.markers[0])


def test_forward_until_tag_default_config() -> None:
    res = parse("Risk: <risk level=high>backend <risk level=low>frontend")
    assert res.segments[1].annotations[0].attrs["level"] == "high"
    assert res.segments[2].annotations[0].attrs["level"] == "low"


def test_unknown_tags_are_stripped() -> None:
    res = parse("Hello <unknown>world</unknown>")
    assert res.text == "Hello world"
    assert all(not seg.annotations for seg in res.segments)


def test_repr_contains_useful_info() -> None:
    res = parse("<cite id=2>Hello</cite>")
    assert "ParseResult" in repr(res)
    assert "segments=" in repr(res)
    seg_repr = repr(res.segments[0])
    assert "Segment" in seg_repr
    assert "Annotation" in repr(res.segments[0].annotations[0])


def test_custom_config_passthrough() -> None:
    cfg = ParserConfig()
    cfg.set_unknown_mode("passthrough")
    cfg.set_recognized_tags(["note"])
    res = parse("Hello <weird>world</weird> <note>ok</note>", cfg)
    assert "weird" in res.text
    assert res.segments[-1].annotations[0].tag == "note"
