from __future__ import annotations

from typing import Dict, List, Literal, Union

AttrValue = Union[bool, str]

class Annotation:
    """Annotation attached to a span."""

    tag: str
    attrs: Dict[str, AttrValue]
    def __repr__(self) -> str: ...
    """Return a readable representation."""

class Segment:
    """A contiguous piece of text plus its covering annotations."""

    text: str
    annotations: List[Annotation]
    def __repr__(self) -> str: ...
    """Return a readable representation."""

class Marker:
    """Zero-width marker emitted by self-closing tags."""

    pos: int
    annotation: Annotation
    def __repr__(self) -> str: ...
    """Return a readable representation."""

class ParseResult:
    """Parsed text, segments, and markers."""

    text: str
    segments: List[Segment]
    markers: List[Marker]
    def __repr__(self) -> str: ...
    """Return a readable summary with counts."""

class ParserConfig:
    """Mutable parser configuration mirrored from Rust."""
    def __init__(self) -> None: ...
    def set_recognized_tags(self, tags: list[str]) -> None: ...
    """Set the whitelist of recognized tags."""
    def set_unknown_mode(
        self, mode: Literal["strip", "passthrough", "treat_as_text"]
    ) -> None: ...
    """How to handle unknown tags: strip markup, passthrough literally, or treat as plain text."""
    def set_recovery_strategy(
        self,
        tag: str,
        strategy: Literal[
            "retro_line",
            "forward_until_tag",
            "forward_until_newline",
            "forward_next_token",
            "noop",
        ],
    ) -> None: ...
    """Override the unclosed-tag recovery for a specific tag."""
    def set_trim_punctuation(self, val: bool) -> None: ...
    """Enable/disable punctuation trimming for retro spans."""
    def set_autoclose_on_any_tag(self, val: bool) -> None: ...
    """Auto-close open tags when any new tag starts."""
    def set_autoclose_on_same_tag(self, val: bool) -> None: ...
    """Auto-close open tags when the same tag is seen again."""
    def set_case_sensitive_tags(self, val: bool) -> None: ...
    """Enable/disable case-sensitive tag matching."""

def parse(text: str, config: ParserConfig = None) -> ParseResult: ...

"""Parse KindaXML text using the default configuration."""

__all__ = ["parse", "Annotation", "Segment", "Marker", "ParseResult"]
