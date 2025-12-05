from __future__ import annotations

from kindaxml import parse


def main() -> None:
    result = parse("We shipped <cite id=1>last week</cite>.")
    print("Text:", result.text)
    for segment in result.segments:
        ann_tags = [a.tag for a in segment.annotations]
        print(f"Segment: {segment.text!r} anns={ann_tags}")
    for marker in result.markers:
        print(f"Marker @{marker.pos}: {marker.annotation.tag}")


if __name__ == "__main__":
    main()
