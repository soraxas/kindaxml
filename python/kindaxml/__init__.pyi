from __future__ import annotations

from typing import Dict, List, Union

AttrValue = Union[bool, str]

class Annotation:
    tag: str
    attrs: Dict[str, AttrValue]
    def __repr__(self) -> str: ...
    """Return a readable representation."""

class Segment:
    text: str
    annotations: List[Annotation]
    def __repr__(self) -> str: ...
    """Return a readable representation."""

class Marker:
    pos: int
    annotation: Annotation
    def __repr__(self) -> str: ...
    """Return a readable representation."""

class ParseResult:
    text: str
    segments: List[Segment]
    markers: List[Marker]
    def __repr__(self) -> str: ...
    """Return a readable summary with counts."""

def parse(text: str) -> ParseResult: ...

"""Parse KindaXML text using the default configuration."""

__all__ = ["parse", "Annotation", "Segment", "Marker", "ParseResult"]

class ParserConfig:
    def __init__(self) -> None: ...
    def set_recognized_tags(self, tags: list[str]) -> None: ...
    def set_unknown_mode(self, mode: str) -> None: ...
    def set_recovery_strategy(self, tag: str, strategy: str) -> None: ...
    def set_trim_punctuation(self, val: bool) -> None: ...
    def set_autoclose_on_any_tag(self, val: bool) -> None: ...
    def set_autoclose_on_same_tag(self, val: bool) -> None: ...
    def set_case_sensitive_tags(self, val: bool) -> None: ...
