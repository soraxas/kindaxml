# Concepts

KindaXML is designed to annotate text without building a deep DOM. The parser emits a flat list of text segments and optional markers, recovering from common model mistakes.

## Core ideas

- **Annotation-first:** tags annotate text spans rather than create nested trees.
- **Deterministic recovery:** malformed tags still map to predictable spans.
- **Shallow structure:** open tags auto-close when the next tag starts to avoid runaway nesting.

## Output model

The parser returns:

- `text`: tag-stripped text
- `segments`: ordered runs of text, each with zero or more annotations
- `markers`: zero-width annotations for self-closing tags

Example (conceptual):

```json
[
  {"text": "We shipped ", "annotations": []},
  {"text": "last week", "annotations": [{"tag": "cite", "attrs": {"id": "1"}}]},
  {"text": ".", "annotations": []}
]
```

## Runnable example

Run:

```bash
cargo run -q --example basic
```

Expected output (excerpt):

```text
=== Inline span ===
Original text:
We shipped <cite id="1">last week</cite>.

Parsed text:
We shipped last week.
Segments:
- 'We shipped '
- 'last week' (cite [id="1"])
- '.'
```

CI tip: compare the excerpt above with the matching section in the command output to verify the annotation model.
