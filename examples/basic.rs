use kindaxml::{AttrValue, ParserConfig, RecoveryStrategy, UnknownMode, parse};
use std::collections::HashSet;

fn main() {
    let config = build_config();
    let samples = vec![
        ("Inline span", "We shipped <cite id=\"1\">last week</cite>."),
        (
            "Retroactive cite",
            "We shipped last week <cite id=1>. More info <note>soon",
        ),
        (
            "Forward token",
            "Risks: <risk level=high> load tests are late. <risk level=low>Docs slipping",
        ),
        (
            "Self-closing markers",
            "Todo list: <todo id=7/>finish rollout <todo/> update docs.",
        ),
    ];

    for (label, input) in samples {
        println!("=== {} ===", label);
        println!("Original text:\n{}\n", input);
        let parsed = parse(input, &config);
        println!("Parsed text:\n{}\nSegments:", parsed.text);

        for segment in &parsed.segments {
            if segment.annotations.is_empty() {
                println!("- '{}'", segment.text);
            } else {
                let anns: Vec<String> = segment
                    .annotations
                    .iter()
                    .map(|ann| {
                        let attrs = format_attrs(&ann.attrs);
                        if attrs.is_empty() {
                            ann.tag.clone()
                        } else {
                            format!("{} [{}]", ann.tag, attrs)
                        }
                    })
                    .collect();
                println!("- '{}' ({})", segment.text, anns.join("; "));
            }
        }

        if !parsed.markers.is_empty() {
            println!("Markers:");
            for marker in &parsed.markers {
                let attrs = format_attrs(&marker.annotation.attrs);
                let tag = if attrs.is_empty() {
                    marker.annotation.tag.clone()
                } else {
                    format!("{} [{}]", marker.annotation.tag, attrs)
                };
                println!("- @{} {}", marker.pos, tag);
            }
        }

        println!();
    }
}

fn build_config() -> ParserConfig {
    let recognized_tags = ["cite", "note", "risk", "todo"]
        .into_iter()
        .map(String::from)
        .collect::<HashSet<_>>();
    let mut cfg = ParserConfig {
        recognized_tags,
        ..ParserConfig::default()
    };
    cfg.per_tag_recovery
        .insert("cite".into(), RecoveryStrategy::RetroLine);
    cfg.per_tag_recovery
        .insert("note".into(), RecoveryStrategy::ForwardUntilNewline);
    cfg.per_tag_recovery
        .insert("risk".into(), RecoveryStrategy::ForwardNextToken);
    cfg.case_sensitive_tags = false;
    cfg.unknown_mode = UnknownMode::Strip;
    cfg
}

fn format_attrs(attrs: &std::collections::HashMap<String, AttrValue>) -> String {
    let mut pairs: Vec<_> = attrs.iter().collect();
    pairs.sort_by_key(|(k, _)| *k);
    pairs
        .into_iter()
        .map(|(k, v)| match v {
            AttrValue::Bool(b) => format!("{}={}", k, b),
            AttrValue::Str(s) => format!("{}=\"{}\"", k, s),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
