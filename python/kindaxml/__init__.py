"""
KindaXML Python bindings.

Use `parse(text, config=None)` to parse annotated text. The default config recognizes
common tags and applies sane recovery; pass a `ParserConfig` to customize:

    from kindaxml import parse, ParserConfig
    cfg = ParserConfig()
    cfg.set_recognized_tags(["cite", "note"])
    cfg.set_unknown_mode("strip")
    result = parse("We shipped <cite id=1>last week</cite>.", cfg)

The module exports native classes: Annotation, Segment, Marker, ParseResult, ParserConfig.
"""

from __future__ import annotations

from typing import TypeAlias

from . import _kindaxml_rs as _native

Annotation: TypeAlias = _native.Annotation
Segment: TypeAlias = _native.Segment
Marker: TypeAlias = _native.Marker
ParseResult: TypeAlias = _native.ParseResult
parse = _native.parse
ParserConfig: TypeAlias = _native.ParserConfig

__all__ = [
    "parse",
    "Annotation",
    "Segment",
    "Marker",
    "ParseResult",
    "ParserConfig",
    "default_llm_friendly",
    "retro_citations_forward_notes",
    "forward_only",
]
