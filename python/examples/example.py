from __future__ import annotations

from kindaxml import parse


def main() -> None:
    input_text = "We shipped <cite id=1>last week</cite>."
    result = parse(input_text)
    print("Original:", input_text)
    print("Text:", result.text)
    for segment in result.segments:
        print(f"Segment: {segment.text!r} anns={segment.annotations}")
    for marker in result.markers:
        print(f"Marker @{marker.pos}: {marker.annotation.tag}")


if __name__ == "__main__":
    main()
