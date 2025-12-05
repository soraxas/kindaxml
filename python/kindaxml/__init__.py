from __future__ import annotations

from dataclasses import dataclass

from . import _kindaxml_rs as kindaxml_rs

__all__ = [
    "parse",
    "Annotation",
    "Segment",
    "Marker",
    "ParseResult",
    "Annotation",
    "Segment",
    "Marker",
    "ParseResult",
]


AttrValue = bool | str


@dataclass
class Annotation:
    tag: str
    attrs: dict[str, AttrValue]


@dataclass
class Segment:
    text: str
    annotations: list[Annotation]


@dataclass
class Marker:
    pos: int
    annotation: Annotation


@dataclass
class ParseResult:
    text: str
    segments: list[Segment]
    markers: list[Marker]

    @classmethod
    def from_native(cls, native: kindaxml_rs.ParseResult) -> "ParseResult":
        """Convert from native Rust ParseResult to typed Python ParseResult."""
        segs = [
            Segment(
                text=s.text,
                annotations=[
                    Annotation(tag=a.tag, attrs=dict(a.attrs)) for a in s.annotations
                ],
            )
            for s in native.segments
        ]
        markers = [
            Marker(
                pos=m.pos,
                annotation=Annotation(
                    tag=m.annotation.tag, attrs=dict(m.annotation.attrs)
                ),
            )
            for m in native.markers
        ]
        return cls(text=native.text, segments=segs, markers=markers)


def parse(text: str) -> ParseResult:
    """
    Parse KindaXML text with the default configuration.

    Returns a typed ParseResult wrapper around the native Rust types.
    """
    native = kindaxml_rs.parse(text)
    return ParseResult.from_native(native)
