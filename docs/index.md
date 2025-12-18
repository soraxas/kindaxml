# kindaxml

KindaXML is a tolerant, XML-ish annotation DSL for LLM output. It favors predictable recovery over strict well-formedness, producing a flat stream of annotated text segments and zero-width markers.

## Quickstart (Rust)

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

## Quickstart (Python)

```python
from kindaxml import parse, ParserConfig

cfg = ParserConfig()
cfg.set_recognized_tags(["cite", "note"])
cfg.set_unknown_mode("strip")

parsed = parse("We shipped <cite id=1>last week</cite>.", cfg)
print(parsed.text)
```

## What to read next

- Concepts: how the model is represented and why it is tolerant.
- Modes: how unknown tags and recovery strategies are configured.
- Examples: runnable examples with expected output.
