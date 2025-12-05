from __future__ import annotations

from typing import TypeAlias

from . import _kindaxml_rs as _native

Annotation: TypeAlias = _native.Annotation
Segment: TypeAlias = _native.Segment
Marker: TypeAlias = _native.Marker
ParseResult: TypeAlias = _native.ParseResult
parse = _native.parse

__all__ = ["parse", "Annotation", "Segment", "Marker", "ParseResult"]
