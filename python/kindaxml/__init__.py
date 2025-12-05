from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List, Union

from ._lib_name import Annotation, Marker, ParseResult, Segment, parse as _parse

__all__ = [
    "parse",
    "PyAnnotation",
    "PySegment",
    "PyMarker",
    "PyParseResult",
    "Annotation",
    "Segment",
    "Marker",
    "ParseResult",
]


AttrValue = Union[bool, str]


@dataclass
class PyAnnotation:
    tag: str
    attrs: Dict[str, AttrValue]


@dataclass
class PySegment:
    text: str
    annotations: List[PyAnnotation]


@dataclass
class PyMarker:
    pos: int
    annotation: PyAnnotation


@dataclass
class PyParseResult:
    text: str
    segments: List[PySegment]
    markers: List[PyMarker]

    @classmethod
    def from_native(cls, native: ParseResult) -> "PyParseResult":
        segs = [
            PySegment(
                text=s.text,
                annotations=[
                    PyAnnotation(tag=a.tag, attrs=dict(a.attrs)) for a in s.annotations
                ],
            )
            for s in native.segments
        ]
        markers = [
            PyMarker(
                pos=m.pos,
                annotation=PyAnnotation(
                    tag=m.annotation.tag, attrs=dict(m.annotation.attrs)
                ),
            )
            for m in native.markers
        ]
        return cls(text=native.text, segments=segs, markers=markers)


def parse(text: str) -> PyParseResult:
    """
    Parse KindaXML text with the default configuration.

    Returns a typed PyParseResult wrapper around the native Rust types.
    """
    native = _parse(text)
    return PyParseResult.from_native(native)
