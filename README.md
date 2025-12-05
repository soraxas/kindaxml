# `kindaxml`, a close-enough, XML-ish markup for LLM output

KindaXML is an **XML-inspired annotation DSL** designed for **LLM-generated text**. It keeps the familiar `<tag attr=...>` shape, but the parser is **tolerant**: it recovers from missing end tags, missing quotes, and other common “almost XML” mistakes.

KindaXML is **not XML** (and not meant to be parsed by strict XML parsers). Think: *well-formed-ish*.

## Why KindaXML?

LLMs are good at emitting XML-like text, but strict XML breaks easily. KindaXML aims to be:

* **LLM-friendly**: angle brackets and attributes feel natural in prompts.
* **Deterministic recovery**: malformed input still produces predictable output.
* **Annotation-first**: tags annotate spans of text rather than building a complex DOM.
* **Configurable**: recognized tags are whitelisted, unknown tags can be stripped or preserved.

## Design: Annotation DSL (Option A) + a pinch of “blocks”

KindaXML’s primary output is a **stream of text segments**, each optionally annotated:

```json
[
  {"text": "We shipped last week", "ann": [{"tag":"cite","attrs":{"id":"1"}}]},
  {"text": ". ", "ann": []},
  {"text": "Details", "ann": [{"tag":"note","attrs":{}}]}
]
```

KindaXML intentionally avoids deep nesting. In fact, it auto-closes open tags when the next tag begins, which keeps structures shallow and robust.

## Syntax overview

### Tags

* Start tag: `<tag ...>`
* End tag: `</tag>`
* Self-closing tag: `<tag .../>`

Tag names match:

```
[A-Za-z][A-Za-z0-9_\-:.]*
```

### Attributes

Supported forms:

* `a="x"`
* `a='x'`
* `a=x` (unquoted)
* `a` (boolean attribute; implies `true`)
* Whitespace around `=` is allowed.

## Parsing rules (the “close enough” part)

### 1) Tag boundary detection

A tag begins at `<` and ends at the first `>`.

If a quote starts inside the tag but never closes, it is **implicitly closed at `>`**.

Example:

```
<cite id='1,2>text</cite>
```

Parses as:

* `tag = cite`
* `id = "1,2"` (quote recovered)
* inner text = `text`

### 2) Auto-close on encountering another tag

If a start tag is open and the parser encounters the next `<something...>`, the current tag is **implicitly closed immediately before** that next `<`.

This is the core rule that prevents runaway structures.

Example:

```
<A>hello <B>world</B>
```

`<A>` auto-closes before `<B>`.

### 3) Missing end tags are tolerated

If a tag never closes, it’s recovered according to its configured **span strategy** (below).

### 4) Self-closing tags

`<tag .../>` is treated as a **marker annotation** at that position (or optionally “annotate next token”, configurable).

## Span strategies (how KindaXML decides what a tag annotates)

KindaXML is annotation-first. Each recognized tag can be configured with a span strategy:

### `inline` (normal XML-ish)

If `<tag> ... </tag>` is present, annotate the inner range.

### `retro_line` (great for citations)

If `<cite ...>` is unclosed, annotate the text on the current line **before** the tag (from last emitted newline to the tag start), optionally trimming punctuation/whitespace.

Example:

```
We shipped last week <cite id=1>.
```

The cite attaches to `We shipped last week` (not the punctuation).

### Other useful strategies (optional)

* `forward_until_tag`: annotate from the end of `<tag ...>` to the next tag start.
* `forward_until_newline`: annotate until newline.
* `forward_next_token`: annotate the next token/word.
* `noop`: ignore tag if unclosed (marker-only tags).

## Unknown tags

You instruct the LLM to use a whitelist of recognized tags, but the parser can handle unknown tags in one of three modes:

* `strip` (default-friendly): drop unknown tag markup, keep inner text
* `passthrough`: keep unknown tags as literal text
* `treat_as_text`: don’t parse unknown tags at all; treat `<...>` as text

## Escaping / literal text (CDATA support)

KindaXML can support XML’s CDATA form:

* Start: `<![CDATA[`
* End: `]]>`

Inside CDATA, nothing is parsed as tags.

Example:

```xml
<note><![CDATA[
Use < and > freely here. Even <fake tags>.
]]></note>
```

If `]]>` is missing, CDATA runs to end-of-document (recovered).

(If you prefer simpler escaping, you can also support `\<` and `\>` as literals.)

## Using the Rust crate

```rust
use kindaxml::{parse, ParserConfig, UnknownMode};

fn main() {
    let mut cfg = ParserConfig::default();
    cfg.recognized_tags = ["cite", "note"].into_iter().map(String::from).collect();
    cfg.case_sensitive_tags = false;
    cfg.unknown_mode = UnknownMode::Strip;

    let input = "We shipped <cite id=1>last week</cite>.";
    let parsed = parse(input, &cfg);

    for segment in parsed.segments {
        println!("{:?} -> {:?}", segment.text, segment.annotations);
    }
}
```

## Python bindings

The Python module is built with `maturin` (`--features python`). Basic usage:

```python
from kindaxml import parse

result = parse("We shipped <cite id=1>last week</cite>.")
print(result.text)
```

To customize parsing, pass a `ParserConfig`:

```python
from kindaxml import parse, ParserConfig

cfg = ParserConfig()
cfg.set_recognized_tags(["cite", "note", "todo"])
cfg.set_unknown_mode("strip")  # or passthrough / treat_as_text
cfg.set_recovery_strategy("cite", "retro_line")
cfg.set_autoclose_on_any_tag(True)

result = parse("We shipped <cite id=1>last week</cite>.", cfg)
```

`ParserConfig` setters roughly mirror the Rust config: per-tag recovery strategies (`retro_line`, `forward_until_tag`, `forward_until_newline`, `forward_next_token`, `noop`), punctuation trimming, auto-close toggles, and case sensitivity.

### Full Python configuration example

```python
from kindaxml import parse, ParserConfig

cfg = ParserConfig()
# Only these tags are recognized
cfg.set_recognized_tags(["cite", "note", "risk", "todo"])

# Unknown tags: remove markup but keep inner text
cfg.set_unknown_mode("strip")

# Recovery strategies per tag
cfg.set_recovery_strategy("cite", "retro_line")          # attach backward on the line
cfg.set_recovery_strategy("note", "forward_until_newline")
cfg.set_recovery_strategy("risk", "forward_next_token")

# Auto-close behaviour
cfg.set_autoclose_on_any_tag(True)    # close open tag when any new tag starts
cfg.set_autoclose_on_same_tag(True)   # close when the same tag reappears

# Misc toggles
cfg.set_trim_punctuation(True)        # trim punctuation for retro spans
cfg.set_case_sensitive_tags(False)    # treat tags case-insensitively

text = "We shipped last week <cite id=1>. Risks: <risk level=high> perf"
parsed = parse(text, cfg)

print(parsed.text)  # tag-stripped text
for seg in parsed.segments:
    print(seg, seg.annotations)
for marker in parsed.markers:
    print(marker)
```

`ParserConfig` exposes toggles for unknown tags, per-tag recovery strategies, case sensitivity, punctuation trimming, and auto-close behavior. The default config is conservative and strips unknown tags.

## Examples

Run the runnable demo with `cargo run --example basic` to see the original snippets alongside their parsed segments and markers.

### Closed tag (inline span)

Input:

```xml
We shipped <cite id="1">last week</cite>.
```

Output (conceptual):

* `We shipped ` (no annotations)
* `last week` (annotated: cite{id=1})
* `.` (no annotations)

### Unclosed cite (retro_line)

Input:

```xml
We shipped last week <cite id=1>.
```

Output:

* `We shipped last week` (annotated: cite{id=1})
* `.`
* (tag removed)

### Broken quote recovery

Input:

```xml
<cite id='1, 2>Evidence</cite>
```

Recovered as `id="1,2"`.

### Auto-close on next tag

Input:

```xml
alpha <note>bravo <cite id=9> charlie
```

* `<note>` auto-closes before `<cite ...>`
* `<cite>` is unclosed and recovered by its strategy

## Failure cases / limitations (by design)

### Nesting will not behave like XML

KindaXML is not a DOM language. If you try to nest, the “auto-close on next tag” rule will flatten it.

Bad idea:

```xml
<A>outer <B>inner</B> outer</A>
```

KindaXML outcome: `<A>` likely ends before `<B>`, and `</A>` may become stray.

**Guidance:** don’t nest; prefer sibling tags.

### Attribute ambiguity in severely malformed tags

Example:

```xml
<tag a="x y z b=2>
```

KindaXML will recover by closing the quote at `>` and treat the entire remaining text as part of `a`. This is intentional: recovery is bounded to the tag.

**Guidance:** keep attributes simple; use CDATA for messy text.

### Stray end tags

Because auto-close flattens structure, you may get stray `</tag>`. By default, recognized stray end tags are dropped; unknown ones can be passed through (configurable).

## Recommended prompting style for LLMs

Tell the model:

* Use only these tags: `<cite> <note> <todo> <risk> ...` (whitelist)
* Do **not** nest tags
* Prefer postfix citations: `... statement <cite id=1>.`
* Use CDATA for code or text with `<`/`>`: `<![CDATA[ ... ]]>`
