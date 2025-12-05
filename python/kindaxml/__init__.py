from ._lib_name import parse as _parse

__all__ = ["parse"]


def parse(text: str):
    """
    Parse KindaXML text with the default configuration.

    Returns a dict with keys: text, segments (list of {text, annotations}), and markers.
    """
    return _parse(text)
