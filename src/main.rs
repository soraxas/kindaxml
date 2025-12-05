use kindaxml::{parse, ParserConfig, UnknownMode};

fn main() {
    let mut config = ParserConfig::default();
    config.recognized_tags = ["cite", "note", "todo"].iter().map(|s| s.to_string()).collect();
    config.case_sensitive_tags = false;
    config.unknown_mode = UnknownMode::Strip;

    let sample = "We shipped <cite id=1>last week</cite>.\nMore info <note>soon";
    let result = parse(sample, &config);

    println!("Parsed text:\n{}\n", result.text);
    println!("Segments:");
    for segment in result.segments {
        if segment.annotations.is_empty() {
            println!("- '{}' (no annotations)", segment.text);
        } else {
            let anns: Vec<String> = segment
                .annotations
                .iter()
                .map(|a| format!("{} {:?}", a.tag, a.attrs))
                .collect();
            println!("- '{}' [{}]", segment.text, anns.join(", "));
        }
    }
}
