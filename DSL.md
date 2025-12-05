# KindaXML Spec + Recovery Rules

This document defines a tolerant, XML-ish annotation DSL designed for LLM output. It is **not XML** and does not aim for a fully-correct DOM. Its purpose is to let a model express structured annotations while remaining **robust to common model formatting errors**.

---

## 1) Goals & Non-Goals

### Goals

* **LLM-friendly syntax**: looks like XML so models naturally emit it.
* **Tolerant parsing**: missing quotes, missing close-tags, and other small errors are recoverable.
* **Annotation-first**: tags primarily **annotate spans** (Option A), with a small amount of “block segmentation” (Option B) via auto-close-on-next-tag.
* **Deterministic recovery**: every malformed input has a predictable parse.

### Non-Goals

* No requirement to preserve XML well-formedness.
* No guarantee of building a faithful nested tree (Option C).
* No strict validation of attribute correctness beyond a simple tokenizer + recovery.

---

## 2) Core Concepts

### 2.1 Tokens

The language recognizes:

* **Start tag**: `<Tag ...>`
* **End tag**: `</Tag>`
* **Self-closing tag**: `<Tag .../>`
* **Text**: any content not part of tags

### 2.2 Tag Names

* `Tag` matches: `[A-Za-z][A-Za-z0-9_\-:.]*`
* Tags are **case-sensitive by default** (configurable).

### 2.3 Recognized vs Unrecognized Tags

The parser operates with a configurable set:

* **Recognized tags**: tags the LLM is instructed to use (e.g., `cite`, `claim`, `todo`, `metric`, `risk`, `note`, etc.)
* **Unrecognized tags**: everything else

Parser behavior is configurable:

* `unknown_mode = "strip"`: remove unknown tags but keep their inner text (if any)
* `unknown_mode = "passthrough"`: leave unknown tags in-place as literal text
* `unknown_mode = "treat_as_text"`: treat `<...>` as plain text if the tag name isn’t recognized

---

## 3) Grammar (Informal)

This is an *informal grammar* describing intended forms; recovery rules (Section 6) define what happens when input deviates.

```
Document  := (Text | Element)*

Element   := StartTag Document? EndTag?
          | SelfClosingTag

StartTag  := "<" Name Attrs? ">"
EndTag    := "</" Name ">"
SelfClosingTag := "<" Name Attrs? "/>"

Attrs     := (WS Attr)*
Attr      := Name (WS? "=" WS? Value)?
Value     := Quoted | Unquoted
Quoted    := "'" (not-quote-or-recovered)* "'"  |  '"' (not-quote-or-recovered)* '"'
Unquoted  := (not-WS-and-not-">" and not "/>")+
```

**Important:** While this grammar allows nesting, **the semantic model does not rely on nesting.** Auto-close behavior makes it closer to “flat annotations”.

---

## 4) Semantics: What a Tag Means

### 4.1 Annotation Tag (Option A)

A recognized tag may annotate:

* **Inline span** between `<Tag ...>` and `</Tag>` (if it closes properly), or
* A **recovered span** determined by recovery rules (if it does not close properly).

The output form is typically a sequence of segments:

```json
[
  {"text": "some text", "ann": []},
  {"text": "annotated phrase", "ann": [{"tag": "cite", "attrs": {"id": "3"}}]},
  ...
]
```

### 4.2 Soft Block Boundary (Option B-lite)

Because “auto-close on encountering another tag” is always applied, tags naturally become **local annotations** and can also be treated as **block delimiters** in downstream logic if you wish.

Example: If `<note>` often begins a line, you can interpret it as “this line is a note” even if it doesn’t close.

---

## 5) Attributes

### 5.1 Allowed Forms

Attributes support all of:

* `a="x"`
* `a='x'`
* `a=x` (unquoted)
* `a` (boolean attribute; value is `true`)
* Whitespace around `=` is allowed.

### 5.2 Recovery for Broken Quotes

If a quoted value is opened but the closing quote is missing:

* The value is terminated by the **end of the tag** (`>` or `/>`), **not** by scanning arbitrarily far.
* In practice: `ok='es>` parses as `ok = "es"` and the quote is considered implicitly closed at `>`.

### 5.3 Duplicate Attributes

If the same attribute appears multiple times:

* Default: **last one wins**
* Configurable alternative: first wins / keep list.

---

## 6) Recovery Rules (Deterministic)

Recovery rules are applied in this order.

### 6.1 Tag Boundary Detection

A tag starts at `<` and ends at the first `>` that is not clearly “inside a quoted value”.

Because we tolerate broken quotes, “inside quoted value” is determined with a *bounded rule*:

* If a quote (`'` or `"`) begins inside a tag but no matching closing quote appears before `>`, the quote is **implicitly closed at `>`**.

This prevents runaway parsing across lines.

### 6.2 Auto-close on Encountering Another Tag

**Rule:** When a start tag `<A ...>` is open and the parser encounters the next tag start `<B...` (recognized or not, configurable), then `<A>` is **implicitly closed immediately before `<B`**, unless `<A>` was explicitly closed earlier.

This is the key rule that keeps the structure flat and robust.

Notes:

* This applies even if the earlier tag was intended to wrap multiple tags.
* This deliberately sacrifices nesting to preserve determinism.

### 6.3 Missing End Tag

If `<A ...>` is opened and no `</A>` appears before:

* end of document, or
* next tag start that forces auto-close (6.2),
  then it is treated as an **unclosed tag** and recovered as described in 6.5.

### 6.4 Self-closing Tags

`<A .../>` produces an annotation event with **zero-length span** (or a “marker” annotation), depending on consumer needs:

* Default: emit `A` as a marker at that position.
* Configurable: treat as annotating the next token / next word / until newline.

### 6.5 Annotation Span Selection for Unclosed Tags

For unclosed tags, the parser chooses an annotation target span using one of these strategies (configurable per tag; defaults shown):

**Default (line-anchored retroactive):**

* Annotate the text on the **current output line** from the most recent emitted newline up to the tag start.
* Optionally trim leading/trailing punctuation/whitespace.

This matches your cited-text design and is excellent for “postfix tags” like citations.

**Alternative strategies (supported by config):**

* `forward_until_tag`: annotate from tag end to next tag start / newline.
* `forward_next_token`: annotate the next contiguous token after the tag.
* `noop`: ignore unclosed tags completely (only for markers).

Recommended defaults by tag type:

* `<cite id=...>`: **retroactive line-anchored**
* `<label ...>`: **forward_next_token** or **retroactive** depending on style
* `<note>`: **forward_until_tag** or **until_newline**

### 6.6 Unknown Tags

If a tag name is unknown:

* `passthrough`: keep it as literal text (no structural meaning)
* `strip`: remove the tag syntax but keep its inner text
* `treat_as_text`: treat the entire `<...>` as text and do not attempt attribute parsing

### 6.7 Malformed End Tags

If `</X>` is found and there is no matching open start tag currently relevant (because of auto-close policy):

* Treat it as literal text **or** drop it (configurable).
  Default: **drop** if `X` is recognized; **passthrough** otherwise.

---

## 7) Escaping and Literal Text (`CDATA` / Alternatives)

### 7.1 Is CDATA “well-known” to LLMs?

Yes—`<![CDATA[ ... ]]>` is a common XML construct and most LLMs trained on web/code corpora will recognize it.

### 7.2 CDATA Support (Recommended)

Support the following literal block:

* Start: `<![CDATA[`
* End: `]]>`

Inside CDATA:

* No tags are parsed; everything is literal text.

If the end delimiter `]]>` is missing:

* Recover by treating CDATA as running to end-of-document.
* (Optional safer recovery) run to the next line that begins with `]]>`.

### 7.3 Simpler Alternative (If you want)

If you prefer not to implement CDATA, support backslash escaping:

* `\<` means literal `<`
* `\>` means literal `>`

This is simpler but less “standard”.

---

## 8) Output Model

A recommended canonical output for consumers:

### 8.1 Segmented Text Stream

Return an ordered list of segments, each with:

* `text: str`
* `annotations: List[Annotation]`

Where each annotation has:

* `tag: str`
* `attrs: dict[str, str|bool]`
* optional `confidence/recovery_reason` metadata

### 8.2 Marker Events (optional)

For self-closing tags or zero-width tags:

* Represent as `{pos, tag, attrs}` events, or
* Convert them into segments with empty `text`.

---

## 9) Examples

### 9.1 Properly Closed

Input:

```
We shipped <cite id="1">last week</cite>.
```

Output:

* “We shipped ” (no ann)
* “last week” (cite id=1)
* “.” (no ann)

### 9.2 Unclosed + Auto-close by Next Tag

Input:

```
We shipped last week <cite id=1> <note>Details...</note>
```

Process:

* `<cite>` is unclosed; next `<note>` forces auto-close.
* Cite applies retroactively to “We shipped last week ” (trim punctuation optional).

### 9.3 Broken Quote in Attribute

Input:

```
<cite id='1, 2>Evidence</cite>
```

Recovery:

* `id` parses as `1, 2` and quote closes at `>`
* emits inner “Evidence” annotated with cite(id=1,2)

### 9.4 Unknown Tag Passthrough

Input:

```
Hello <weird x=1>world</weird>
```

With `unknown_mode=passthrough`, output keeps `<weird x=1>` as literal text.

### 9.5 CDATA Literal

Input:

```
<note><![CDATA[Use < and > freely here]]></note>
```

No tag parsing inside CDATA.

---

## 10) Failure Cases & Known Limitations (By Design)

### 10.1 Nested Intent Will Flatten

Input:

```
<A>outer <B>inner</B> more</A>
```

Because you always auto-close when encountering other tags, the parser may treat:

* `<A>` closes before `<B>`
* `</A>` may be dropped as unmatched

**Guidance to LLM:** avoid nesting; prefer sibling tags.

### 10.2 Attribute Edge Weirdness

Input:

```
<tag a="x y z b=2>
```

Recovery rule closes quote at `>` so `a="x y z b=2` (probably not intended).
Mitigation:

* Encourage quoted attributes to end before `>` and discourage embedding `b=` in values.

### 10.3 Literal “<” Without CDATA/Escape

If CDATA/escaping isn’t used, `<` can start a tag accidentally.
Mitigation:

* Recommend CDATA for code snippets and angle-bracket-heavy text.

### 10.4 Stray `</tag>` closers

Given auto-close, closers often become stray.
Mitigation:

* Default to drop recognized stray closers.

---

## 11) Recommended Authoring Guidelines (for the LLM Prompt)

Tell the LLM:

* Use only recognized tags: `<cite> <note> <risk> <todo> ...`
* Do not nest tags.
* Prefer putting `<cite ...>` at end of line (postfix) if using retroactive mode.
* Use `<![CDATA[ ... ]]>` for code or any text containing `<` `>`.

---

## 12) Configuration Surface (Suggested)

* `recognized_tags: set[str]`
* `unknown_mode: {"strip","passthrough","treat_as_text"}`
* `autoclose_on_any_tag: bool` (you want true)
* `per_tag_recovery_strategy: dict[tag -> strategy]`

  * `retro_line` (default for cite)
  * `forward_until_tag`
  * `forward_until_newline`
  * `forward_next_token`
  * `noop`
* `trim_punctuation: bool`
* `case_sensitive_tags: bool`
* `stray_end_tag_policy: {"drop","passthrough"}`
