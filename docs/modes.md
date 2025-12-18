# Modes

Modes control how KindaXML recovers from malformed input and how it treats unknown tags.

## Unknown tag handling

`UnknownMode` determines what happens when the parser sees a tag that is not in `recognized_tags`:

- `Strip`: remove the tag markup but keep inner text
- `Passthrough`: keep the literal tag markup in the output text
- `TreatAsText`: do not treat `<...>` as a tag at all

## Recovery strategies

Unclosed tags use `RecoveryStrategy` (configurable per tag):

- `RetroLine`: annotate text on the same line before the tag
- `ForwardUntilTag`: annotate until the next tag start
- `ForwardUntilNewline`: annotate until the next newline
- `ForwardNextToken`: annotate the next token only
- `Noop`: ignore the unclosed tag

## Autoclose and stray end tags

- `autoclose_on_any_tag`: close open tags when any new tag starts
- `autoclose_on_same_tag`: close an open tag if the same tag appears again
- `stray_end_tag_policy`: drop or passthrough unmatched end tags

## Runnable example

Run:

```bash
cargo run -q --example basic
```

Expected output (excerpt):

```text
=== Forward token ===
Original text:
Risks: <risk level=high> load tests are late. <risk level=low>Docs slipping

Parsed text:
Risks:  load tests are late. Docs slipping
Segments:
- 'Risks: '
- ' load tests are late. ' (risk [level="high"])
- 'Docs' (risk [level="low"])
- ' slipping'
```

CI tip: this section shows `ForwardNextToken` and can be validated by diffing the output against the excerpt above.
