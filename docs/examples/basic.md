# Basic example

This page mirrors `examples/basic.rs` and shows the full output for easy CI verification.

Run:

```bash
cargo run -q --example basic
```

Expected output:

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

=== Retroactive cite ===
Original text:
We shipped last week <cite id=1>. More info <note>soon

Parsed text:
We shipped last week . More info soon
Segments:
- 'We shipped last week' (cite [id="1"])
- ' . More info '
- 'soon' (note)

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

=== Self-closing markers ===
Original text:
Todo list: <todo id=7/>finish rollout <todo/> update docs.

Parsed text:
Todo list: finish rollout  update docs.
Segments:
- 'Todo list: finish rollout  update docs.'
Markers:
- @11 todo [id="7"]
- @26 todo
```
