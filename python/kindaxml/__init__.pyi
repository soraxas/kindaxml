from __future__ import annotations

from typing import Dict, List, Union

AttrValue = Union[bool, str]

class Annotation:
    tag: str
    attrs: Dict[str, AttrValue]
    def __repr__(self) -> str: ...

class Segment:
    text: str
    annotations: List[Annotation]
    def __repr__(self) -> str: ...

class Marker:
    pos: int
    annotation: Annotation
    def __repr__(self) -> str: ...

class ParseResult:
    text: str
    segments: List[Segment]
    markers: List[Marker]
    def __repr__(self) -> str: ...

def parse(text: str) -> ParseResult: ...

__all__ = ["parse", "Annotation", "Segment", "Marker", "ParseResult"]
