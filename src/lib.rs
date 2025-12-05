use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValue {
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    pub tag: String,
    pub attrs: HashMap<String, AttrValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    pub pos: usize,
    pub annotation: Annotation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownMode {
    Strip,
    Passthrough,
    TreatAsText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStrategy {
    RetroLine,
    ForwardUntilTag,
    ForwardUntilNewline,
    ForwardNextToken,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrayEndTagPolicy {
    Drop,
    Passthrough,
}

#[derive(Debug, Clone)]
pub struct ParserConfig {
    pub recognized_tags: HashSet<String>,
    pub per_tag_recovery: HashMap<String, RecoveryStrategy>,
    pub unknown_mode: UnknownMode,
    pub autoclose_on_any_tag: bool,
    pub autoclose_on_same_tag: bool,
    pub trim_punctuation: bool,
    pub case_sensitive_tags: bool,
    pub stray_end_tag_policy: StrayEndTagPolicy,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            recognized_tags: HashSet::new(),
            per_tag_recovery: HashMap::new(),
            unknown_mode: UnknownMode::Strip,
            autoclose_on_any_tag: true,
            autoclose_on_same_tag: true,
            trim_punctuation: true,
            case_sensitive_tags: true,
            stray_end_tag_policy: StrayEndTagPolicy::Drop,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    pub text: String,
    pub segments: Vec<Segment>,
    pub markers: Vec<Marker>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TagKind {
    Start,
    End,
    SelfClosing,
}

#[derive(Debug, Clone)]
struct TagToken {
    raw: String,
    name: String,
    normalized_name: String,
    attrs: HashMap<String, AttrValue>,
    kind: TagKind,
}

#[derive(Debug, Clone)]
struct OpenTag {
    name: String,
    normalized_name: String,
    attrs: HashMap<String, AttrValue>,
    start_pos: usize,
    line_start_at_open: usize,
    strategy: RecoveryStrategy,
}

pub fn parse(input: &str, config: &ParserConfig) -> ParseResult {
    let mut parser = Parser::new(input, config);
    parser.run();
    parser.finish()
}

struct Parser<'a> {
    input: &'a str,
    config: &'a ParserConfig,
    recognized: HashSet<String>,
    per_tag_recovery: HashMap<String, RecoveryStrategy>,
    text: String,
    markers: Vec<Marker>,
    spans: Vec<(usize, usize, Annotation)>,
    open: Vec<OpenTag>,
    line_start: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, config: &'a ParserConfig) -> Self {
        let recognized = if config.case_sensitive_tags {
            config.recognized_tags.clone()
        } else {
            config
                .recognized_tags
                .iter()
                .map(|t| t.to_ascii_lowercase())
                .collect()
        };

        let per_tag_recovery = if config.case_sensitive_tags {
            config.per_tag_recovery.clone()
        } else {
            config
                .per_tag_recovery
                .iter()
                .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
                .collect()
        };

        Self {
            input,
            config,
            recognized,
            per_tag_recovery,
            text: String::new(),
            markers: Vec::new(),
            spans: Vec::new(),
            open: Vec::new(),
            line_start: 0,
        }
    }

    fn finish(mut self) -> ParseResult {
        let end_pos = self.text.len();
        self.close_all_open(end_pos);
        let segments = self.build_segments();

        ParseResult {
            text: self.text,
            segments,
            markers: self.markers,
        }
    }

    fn run(&mut self) {
        let mut idx = 0;
        let bytes = self.input.as_bytes();
        while idx < self.input.len() {
            if self.input[idx..].starts_with("<![CDATA[") {
                let cdata_start = idx + "<![CDATA[".len();
                if let Some(end) = self.input[cdata_start..].find("]]>") {
                    let literal_end = cdata_start + end;
                    let literal = &self.input[cdata_start..literal_end];
                    self.push_text(literal);
                    idx = literal_end + 3;
                } else {
                    let literal = &self.input[cdata_start..];
                    self.push_text(literal);
                    idx = self.input.len();
                }
                continue;
            }

            if bytes[idx] == b'<'
                && let Some((token, consumed)) = self.parse_tag(idx) {
                    if self.should_treat_as_text(&token) {
                        self.push_text(&token.raw);
                        idx += consumed;
                        continue;
                    }

                    match token.kind {
                        TagKind::Start => {
                            if self.is_recognized(&token.normalized_name) {
                                self.maybe_autoclose_on_start_like(&token.normalized_name);
                            }
                            self.handle_start(token);
                        }
                        TagKind::SelfClosing => {
                            if self.is_recognized(&token.normalized_name) {
                                self.maybe_autoclose_on_start_like(&token.normalized_name);
                            }
                            self.handle_self_closing(token);
                        }
                        TagKind::End => {
                            self.handle_end(token);
                        }
                    }
                    idx += consumed;
                    continue;
                }

            if let Some(next_lt) = self.input[idx + 1..].find('<') {
                let slice = &self.input[idx..idx + 1 + next_lt];
                self.push_text(slice);
                idx += 1 + next_lt;
            } else {
                let slice = &self.input[idx..];
                self.push_text(slice);
                idx = self.input.len();
            }
        }
    }

    fn parse_tag(&self, start: usize) -> Option<(TagToken, usize)> {
        let remaining = &self.input[start..];
        let mut in_quote: Option<char> = None;
        let mut end_offset: Option<usize> = None;
        for (i, ch) in remaining.char_indices() {
            match ch {
                '\'' | '"' => {
                    if in_quote == Some(ch) {
                        in_quote = None;
                    } else if in_quote.is_none() {
                        in_quote = Some(ch);
                    }
                }
                '>' => {
                    end_offset = Some(i);
                    break;
                }
                _ => {}
            }
        }

        let end_offset = end_offset?;
        let raw = &remaining[..=end_offset];
        if raw.len() < 3 {
            return None;
        }

        let inner = &raw[1..raw.len() - 1];
        let mut trimmed = inner.trim();

        let kind = if trimmed.starts_with('/') {
            trimmed = trimmed[1..].trim_start();
            TagKind::End
        } else {
            TagKind::Start
        };

        let mut self_closing = false;
        if matches!(kind, TagKind::Start) {
            let without_trailing = trimmed.trim_end();
            if let Some(stripped) = without_trailing.strip_suffix('/') {
                self_closing = true;
                trimmed = stripped.trim_end();
            } else {
                trimmed = without_trailing;
            }
        }

        let (name, rest) = parse_name_and_rest(trimmed)?;
        let attrs = if matches!(kind, TagKind::Start) {
            parse_attrs(rest)
        } else {
            HashMap::new()
        };

        let normalized_name = self.normalize_tag(&name);
        let final_kind = if self_closing {
            TagKind::SelfClosing
        } else {
            kind
        };

        Some((
            TagToken {
                raw: raw.to_string(),
                name,
                normalized_name,
                attrs,
                kind: final_kind,
            },
            raw.len(),
        ))
    }

    fn normalize_tag(&self, name: &str) -> String {
        if self.config.case_sensitive_tags {
            name.to_string()
        } else {
            name.to_ascii_lowercase()
        }
    }

    fn handle_start(&mut self, token: TagToken) {
        let recognized = self.is_recognized(&token.normalized_name);
        if !recognized {
            match self.config.unknown_mode {
                UnknownMode::Strip => {}
                UnknownMode::Passthrough | UnknownMode::TreatAsText => {
                    self.push_text(&token.raw);
                }
            }
            return;
        }

        let strategy = self
            .per_tag_recovery
            .get(&token.normalized_name)
            .cloned()
            .unwrap_or(RecoveryStrategy::RetroLine);

        let open = OpenTag {
            name: token.name,
            normalized_name: token.normalized_name,
            attrs: token.attrs,
            start_pos: self.text.len(),
            line_start_at_open: self.line_start,
            strategy,
        };
        self.open.push(open);
    }

    fn handle_self_closing(&mut self, token: TagToken) {
        let recognized = self.is_recognized(&token.normalized_name);
        if !recognized {
            match self.config.unknown_mode {
                UnknownMode::Strip => {}
                UnknownMode::Passthrough | UnknownMode::TreatAsText => {
                    self.push_text(&token.raw);
                }
            }
            return;
        }

        let annotation = Annotation {
            tag: token.name,
            attrs: token.attrs,
        };
        let marker = Marker {
            pos: self.text.len(),
            annotation,
        };
        self.markers.push(marker);
    }

    fn handle_end(&mut self, token: TagToken) {
        let recognized = self.is_recognized(&token.normalized_name);
        if !recognized {
            match self.config.unknown_mode {
                UnknownMode::Strip => {}
                UnknownMode::Passthrough | UnknownMode::TreatAsText => {
                    self.push_text(&token.raw);
                }
            }
            return;
        }

        if let Some(idx) = self
            .open
            .iter()
            .rposition(|o| o.normalized_name == token.normalized_name)
        {
            let close_pos = self.text.len();
            // Close any newer tags first using recovery.
            let trailing = self.open.split_off(idx + 1);
            for t in trailing.into_iter().rev() {
                self.close_tag(t, close_pos);
            }

            if let Some(open) = self.open.pop() {
                self.close_explicit(open, close_pos);
            }
        } else {
            match self.config.stray_end_tag_policy {
                StrayEndTagPolicy::Drop => {}
                StrayEndTagPolicy::Passthrough => self.push_text(&token.raw),
            }
        }
    }

    fn close_all_open(&mut self, close_pos: usize) {
        while let Some(open) = self.open.pop() {
            self.close_tag(open, close_pos);
        }
    }

    fn close_same_tag(&mut self, normalized_name: &str, close_pos: usize) {
        if let Some(idx) = self
            .open
            .iter()
            .rposition(|o| o.normalized_name == normalized_name)
        {
            let trailing = self.open.split_off(idx + 1);
            for t in trailing.into_iter().rev() {
                self.close_tag(t, close_pos);
            }
            if let Some(open) = self.open.pop() {
                self.close_explicit(open, close_pos);
            }
        }
    }

    fn close_explicit(&mut self, open: OpenTag, close_pos: usize) {
        if open.start_pos >= close_pos {
            return;
        }
        let annotation = Annotation {
            tag: open.name,
            attrs: open.attrs,
        };
        self.spans.push((open.start_pos, close_pos, annotation));
    }

    fn close_tag(&mut self, open: OpenTag, close_pos: usize) {
        match open.strategy {
            RecoveryStrategy::Noop => (),
            RecoveryStrategy::RetroLine => {
                let mut start = open.line_start_at_open;
                let end = open.start_pos;
                if start > end {
                    start = end;
                }
                let (start, end) = self.trim_span(start, end);
                if start < end {
                    let annotation = Annotation {
                        tag: open.name,
                        attrs: open.attrs,
                    };
                    self.spans.push((start, end, annotation));
                }
            }
            RecoveryStrategy::ForwardUntilTag => {
                let mut end = close_pos;
                if let Some(rel) = self.text[open.start_pos..close_pos].find('\n') {
                    end = open.start_pos + rel;
                }
                self.push_forward_span(&open, open.start_pos, end);
            }
            RecoveryStrategy::ForwardUntilNewline => {
                let mut end = close_pos;
                if let Some(rel) = self.text[open.start_pos..close_pos].find('\n') {
                    end = open.start_pos + rel;
                }
                self.push_forward_span(&open, open.start_pos, end);
            }
            RecoveryStrategy::ForwardNextToken => {
                let slice = &self.text[open.start_pos..close_pos];
                if let Some((token_start, token_end)) = next_token_bounds(slice) {
                    let start = open.start_pos + token_start;
                    let end = open.start_pos + token_end;
                    self.push_forward_span(&open, start, end);
                }
            }
        }
    }

    fn push_forward_span(&mut self, open: &OpenTag, mut start: usize, mut end: usize) {
        if start > end {
            std::mem::swap(&mut start, &mut end);
        }
        let (start, end) = self.trim_span(start, end);
        if start < end {
            let annotation = Annotation {
                tag: open.name.clone(),
                attrs: open.attrs.clone(),
            };
            self.spans.push((start, end, annotation));
        }
    }

    fn build_segments(&self) -> Vec<Segment> {
        if self.text.is_empty() {
            return Vec::new();
        }

        let mut bounds: Vec<usize> = vec![0, self.text.len()];
        for (s, e, _) in &self.spans {
            bounds.push(*s);
            bounds.push(*e);
        }
        bounds.sort_unstable();
        bounds.dedup();

        let mut segments = Vec::new();
        for window in bounds.windows(2) {
            let start = window[0];
            let end = window[1];
            if start == end {
                continue;
            }
            let text = self.text[start..end].to_string();
            let annotations = self
                .spans
                .iter()
                .filter(|(s, e, _)| *s <= start && *e >= end && *s != *e)
                .map(|(_, _, ann)| ann.clone())
                .collect();
            segments.push(Segment { text, annotations });
        }

        segments
    }

    fn trim_span(&self, mut start: usize, mut end: usize) -> (usize, usize) {
        if !self.config.trim_punctuation {
            return (start, end);
        }
        if start >= end {
            return (start, end);
        }

        while start < end {
            let ch = self.text[start..].chars().next().unwrap();
            if is_trim_char(ch) {
                start += ch.len_utf8();
            } else {
                break;
            }
        }

        while end > start {
            let ch = self.text[..end].chars().next_back().unwrap();
            if is_trim_char(ch) {
                end -= ch.len_utf8();
            } else {
                break;
            }
        }

        (start, end)
    }

    fn push_text(&mut self, text: &str) {
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                self.line_start = self.text.len() + i + ch.len_utf8();
            }
        }
        self.text.push_str(text);
    }

    fn is_recognized(&self, name: &str) -> bool {
        self.recognized.contains(name)
    }

    fn should_treat_as_text(&self, token: &TagToken) -> bool {
        matches!(self.config.unknown_mode, UnknownMode::TreatAsText)
            && !self.is_recognized(&token.normalized_name)
    }

    fn maybe_autoclose_on_start_like(&mut self, normalized_name: &str) {
        if self.config.autoclose_on_same_tag
            && self
                .open
                .iter()
                .any(|o| o.normalized_name == normalized_name)
        {
            self.close_same_tag(normalized_name, self.text.len());
        }
        if self.config.autoclose_on_any_tag {
            self.close_all_open(self.text.len());
        }
    }
}

fn parse_name_and_rest(input: &str) -> Option<(String, &str)> {
    let mut chars = input.char_indices().peekable();
    if let Some((_, ch)) = chars.peek().copied() {
        if !is_name_start(ch) {
            return None;
        }
    } else {
        return None;
    }

    let mut end_idx = 0;
    for (idx, ch) in input.char_indices() {
        if is_name_continue(ch) {
            end_idx = idx + ch.len_utf8();
        } else {
            break;
        }
    }

    let name = input[..end_idx].to_string();
    let rest = &input[end_idx..];
    Some((name, rest))
}

fn parse_attrs(mut input: &str) -> HashMap<String, AttrValue> {
    let mut attrs = HashMap::new();
    while !input.is_empty() {
        let trimmed = input.trim_start();
        if trimmed.is_empty() {
            break;
        }
        let consumed_ws = input.len() - trimmed.len();
        input = &input[consumed_ws..];

        let mut name = String::new();
        let mut idx = 0;
        for ch in input.chars() {
            if is_name_continue(ch) {
                name.push(ch);
                idx += ch.len_utf8();
            } else {
                break;
            }
        }
        if name.is_empty() {
            break;
        }
        input = &input[idx..];

        let mut after_eq = input.trim_start();
        input = after_eq;
        let mut value: AttrValue = AttrValue::Bool(true);
        if input.starts_with('=') {
            input = &input[1..];
            after_eq = input.trim_start();
            input = after_eq;

            if let Some(first) = input.chars().next() {
                if first == '"' || first == '\'' {
                    let quote = first;
                    input = &input[first.len_utf8()..];
                    if let Some(pos) = input.find(quote) {
                        let val = &input[..pos];
                        value = AttrValue::Str(val.to_string());
                        input = &input[pos + quote.len_utf8()..];
                    } else {
                        // Broken quote: run until end of tag text
                        value = AttrValue::Str(input.to_string());
                        input = "";
                    }
                } else {
                    let mut end = 0;
                    for (i, ch) in input.char_indices() {
                        if ch.is_whitespace() || ch == '/' || ch == '>' {
                            break;
                        }
                        end = i + ch.len_utf8();
                    }
                    if end == 0 && !input.is_empty() {
                        end = input.len();
                    }
                    let val = &input[..end];
                    value = AttrValue::Str(val.to_string());
                    input = &input[end..];
                }
            }
        }

        attrs.insert(name, value);
    }

    attrs
}

fn next_token_bounds(slice: &str) -> Option<(usize, usize)> {
    let mut start = None;
    let mut end = None;
    for (idx, ch) in slice.char_indices() {
        if ch.is_alphanumeric() {
            if start.is_none() {
                start = Some(idx);
            }
            end = Some(idx + ch.len_utf8());
        } else if start.is_some() {
            break;
        }
    }
    match (start, end) {
        (Some(s), Some(e)) => Some((s, e)),
        _ => None,
    }
}

fn is_name_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
}

fn is_name_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.')
}

fn is_trim_char(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, ',' | '.' | ';' | ':' | '!' | '?' | ')' | '(')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn annotated_texts(result: &ParseResult, tag: &str) -> Vec<String> {
        result
            .segments
            .iter()
            .filter(|seg| seg.annotations.iter().any(|a| a.tag == tag))
            .map(|seg| seg.text.clone())
            .collect()
    }

    fn base_config() -> ParserConfig {
        let mut cfg = ParserConfig::default();
        cfg.recognized_tags = ["cite", "note", "todo", "claim", "risk", "code"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        cfg.trim_punctuation = true;
        cfg.case_sensitive_tags = false;
        cfg.per_tag_recovery
            .insert("cite".into(), RecoveryStrategy::RetroLine);
        for tag in ["note", "todo", "claim", "risk", "code"] {
            cfg.per_tag_recovery
                .insert(tag.into(), RecoveryStrategy::ForwardUntilTag);
        }
        cfg
    }

    #[test]
    fn parses_closed_span() {
        let cfg = base_config();
        let result = parse("We shipped <cite id=\"1\">last week</cite>.", &cfg);
        assert_eq!(result.text, "We shipped last week.");
        assert_eq!(result.segments.len(), 3);
        assert!(
            result.segments[1]
                .annotations
                .iter()
                .any(|a| a.tag == "cite")
        );
        let ann = &result.segments[1].annotations[0];
        assert_eq!(ann.attrs.get("id"), Some(&AttrValue::Str("1".into())));
        assert_eq!(result.segments[1].text, "last week");
    }

    #[test]
    fn retroactive_close_on_next_tag() {
        let cfg = base_config();
        let result = parse(
            "We shipped last week <cite id=1> <note>Details...</note>",
            &cfg,
        );
        assert_eq!(result.text.trim_end(), "We shipped last week  Details...");
        let cite = result
            .segments
            .iter()
            .find(|s| s.annotations.iter().any(|a| a.tag == "cite"))
            .expect("cite span");
        assert!(cite.text.contains("We shipped last week"));
    }

    #[test]
    fn recovers_broken_quote() {
        let cfg = base_config();
        let result = parse("<cite id='1, 2>Evidence</cite>", &cfg);
        let cite = result
            .segments
            .iter()
            .find(|s| s.annotations.iter().any(|a| a.tag == "cite"))
            .unwrap();
        let ann = cite.annotations.iter().find(|a| a.tag == "cite").unwrap();
        assert_eq!(ann.attrs.get("id"), Some(&AttrValue::Str("1, 2".into())));
        assert_eq!(cite.text, "Evidence");
    }

    #[test]
    fn unknown_passthrough() {
        let mut cfg = base_config();
        cfg.unknown_mode = UnknownMode::Passthrough;
        let result = parse("Hello <weird x=1>world</weird>", &cfg);
        assert!(result.text.contains("<weird x=1>"));
    }

    #[test]
    fn cdata_literal() {
        let cfg = base_config();
        let result = parse("<note><![CDATA[Use < and > freely here]]></note>", &cfg);
        assert!(result.text.contains("< and >"));
    }

    #[test]
    fn treat_as_text_does_not_autoclose_known_tags() {
        let mut cfg = base_config();
        cfg.unknown_mode = UnknownMode::TreatAsText;

        let result = parse(
            "Risks: <risk level=high>delays <mystery>??</mystery> persist",
            &cfg,
        );

        assert_eq!(result.text, "Risks: delays <mystery>??</mystery> persist");

        let risk_segment = result
            .segments
            .iter()
            .find(|s| s.annotations.iter().any(|a| a.tag == "risk"))
            .expect("risk segment");
        assert_eq!(risk_segment.text, "delays <mystery>??</mystery> persist");

        let ann = risk_segment
            .annotations
            .iter()
            .find(|a| a.tag == "risk")
            .unwrap();
        assert_eq!(ann.attrs.get("level"), Some(&AttrValue::Str("high".into())));
    }

    #[test]
    fn parses_multiple_attributes_and_quotes() {
        let cfg = base_config();
        let result = parse(
            "<claim id=7 confidence=0.62 source='internal'>It works.</claim>",
            &cfg,
        );
        assert_eq!(result.text, "It works.");
        let claim = annotated_texts(&result, "claim");
        assert_eq!(claim, vec!["It works."]);
        let attrs = &result.segments[0].annotations[0].attrs;
        assert_eq!(attrs.get("id"), Some(&AttrValue::Str("7".into())));
        assert_eq!(
            attrs.get("confidence"),
            Some(&AttrValue::Str("0.62".into()))
        );
        assert_eq!(
            attrs.get("source"),
            Some(&AttrValue::Str("internal".into()))
        );
    }

    #[test]
    fn boolean_attribute() {
        let cfg = base_config();
        let result = parse("<todo urgent>Fix flaky test</todo>", &cfg);
        assert_eq!(result.text, "Fix flaky test");
        let todo = annotated_texts(&result, "todo");
        assert_eq!(todo, vec!["Fix flaky test"]);
        let attrs = &result.segments[0].annotations[0].attrs;
        assert_eq!(attrs.get("urgent"), Some(&AttrValue::Bool(true)));
    }

    #[test]
    fn adjacent_tags_keep_spans() {
        let cfg = base_config();
        let result = parse("<cite id=1>A</cite><cite id=2>B</cite>", &cfg);
        assert_eq!(result.text, "AB");
        let cites = annotated_texts(&result, "cite");
        assert_eq!(cites, vec!["A", "B"]);
    }

    #[test]
    fn self_closing_marker_emits_marker() {
        let cfg = base_config();
        let result = parse("Start <todo id=3/> end", &cfg);
        assert_eq!(result.text, "Start  end");
        assert_eq!(result.markers.len(), 1);
        let marker = &result.markers[0];
        assert_eq!(marker.pos, "Start ".len());
        assert_eq!(marker.annotation.tag, "todo");
        assert_eq!(
            marker.annotation.attrs.get("id"),
            Some(&AttrValue::Str("3".into()))
        );
    }

    #[test]
    fn cdata_literal_in_code() {
        let cfg = base_config();
        let result = parse(
            "<code><![CDATA[if (a < b) { return a > 0; }]]></code>",
            &cfg,
        );
        assert_eq!(result.text, "if (a < b) { return a > 0; }");
        let code = annotated_texts(&result, "code");
        assert_eq!(code, vec!["if (a < b) { return a > 0; }"]);
    }

    #[test]
    fn postfix_cite_retro_line() {
        let cfg = base_config();
        let result = parse("We shipped last week <cite id=1>.", &cfg);
        assert_eq!(result.text, "We shipped last week .");
        let cites = annotated_texts(&result, "cite");
        assert_eq!(cites, vec!["We shipped last week"]);
        assert_eq!(result.segments.last().unwrap().text, " .");
    }

    #[test]
    fn retro_line_respects_newline_anchor() {
        let cfg = base_config();
        let result = parse("## Results\nWe shipped last week <cite id=1>.", &cfg);
        assert_eq!(result.text, "## Results\nWe shipped last week .");
        let cites = annotated_texts(&result, "cite");
        assert_eq!(cites, vec!["We shipped last week"]);
    }

    #[test]
    fn retro_line_trims_punctuation() {
        let cfg = base_config();
        let result = parse("We shipped last week, <cite id=1>", &cfg);
        assert_eq!(result.text, "We shipped last week, ");
        let cites = annotated_texts(&result, "cite");
        assert_eq!(cites, vec!["We shipped last week"]);
    }

    #[test]
    fn unclosed_cite_at_end_of_doc() {
        let cfg = base_config();
        let result = parse("We shipped last week <cite id=1>", &cfg);
        assert_eq!(result.text, "We shipped last week ");
        let cites = annotated_texts(&result, "cite");
        assert_eq!(cites, vec!["We shipped last week"]);
    }

    #[test]
    fn unclosed_todo_can_result_in_empty_span() {
        let cfg = base_config();
        let result = parse("Fix this please <todo urgent>\nthanks", &cfg);
        assert_eq!(result.text, "Fix this please \nthanks");
        assert!(annotated_texts(&result, "todo").is_empty());
    }

    #[test]
    fn auto_close_flattens_tags() {
        let cfg = base_config();
        let result = parse("Alpha <note>bravo <cite id=9> charlie", &cfg);
        assert_eq!(result.text, "Alpha bravo  charlie");
        let note = annotated_texts(&result, "note");
        assert_eq!(note, vec!["bravo"]);
        let cites = annotated_texts(&result, "cite");
        assert_eq!(cites.join(""), "Alpha bravo");
    }

    #[test]
    fn unquoted_and_broken_quotes_recover() {
        let cfg = base_config();
        let one = parse("<cite id=1>Evidence</cite>", &cfg);
        let ann = one.segments[0].annotations[0].attrs.get("id");
        assert_eq!(ann, Some(&AttrValue::Str("1".into())));

        let broken_single = parse("<cite id='1,2>Evidence</cite>", &cfg);
        let ann = broken_single.segments[0].annotations[0].attrs.get("id");
        assert_eq!(ann, Some(&AttrValue::Str("1,2".into())));

        let broken_double = parse("<cite id=\"3>Evidence</cite>", &cfg);
        let ann = broken_double.segments[0].annotations[0].attrs.get("id");
        assert_eq!(ann, Some(&AttrValue::Str("3".into())));

        let broken_double_with_other_attr = parse("<cite id=\"4 ok=yes>Evidence</cite>", &cfg);
        let ann = broken_double_with_other_attr.segments[0].annotations[0]
            .attrs
            .get("id");
        assert_eq!(ann, Some(&AttrValue::Str("4 ok=yes".into())));

        let broken_single_with_other_attr = parse("<cite id='5 ok=yes>Evidence</cite>", &cfg);
        let ann = broken_single_with_other_attr.segments[0].annotations[0]
            .attrs
            .get("id");
        assert_eq!(ann, Some(&AttrValue::Str("5 ok=yes".into())));
    }

    #[test]
    fn duplicate_attrs_last_wins() {
        let cfg = base_config();
        let result = parse("<cite id=1 id=2>Evidence</cite>", &cfg);
        let ann = result.segments[0].annotations[0].attrs.get("id");
        assert_eq!(ann, Some(&AttrValue::Str("2".into())));
    }

    #[test]
    #[ignore = "not implemented yet"]
    fn duplicate_attrs_as_comma_list() {
        let _cfg = base_config();
    }

    #[test]
    fn boolean_attr_without_value() {
        let cfg = base_config();
        let result = parse("<cite id>Evidence</cite>", &cfg);
        let ann = result.segments[0].annotations[0].attrs.get("id");
        assert_eq!(ann, Some(&AttrValue::Bool(true)));
    }

    #[test]
    fn missing_gt_treated_as_text() {
        let cfg = base_config();
        let result = parse("We shipped <cite id=1\nyesterday.", &cfg);
        assert!(result.text.contains("<cite id=1\n"));
        assert!(annotated_texts(&result, "cite").is_empty());
    }

    #[test]
    fn unknown_tag_stripped_inner_preserved() {
        let cfg = base_config();
        let result = parse("Hello <weird x=1>world</weird>!", &cfg);
        assert_eq!(result.text, "Hello world!");
        assert!(
            result
                .segments
                .iter()
                .all(|s| s.annotations.iter().all(|a| a.tag != "weird"))
        );
    }

    #[test]
    fn reopening_same_tag_auto_close() {
        let cfg = base_config();
        let result = parse("<cite id=1>One <cite id=2>Two</cite>", &cfg);
        assert_eq!(result.text, "One Two");
        let cites = annotated_texts(&result, "cite");
        assert_eq!(cites, vec!["One ", "Two"]);
    }

    #[test]
    fn stray_closer_dropped_before_unclosed_tag() {
        let cfg = base_config();
        let result = parse("We shipped last week</cite><cite id=1>.", &cfg);
        assert_eq!(result.text, "We shipped last week.");
        let cites = annotated_texts(&result, "cite");
        assert_eq!(cites, vec!["We shipped last week"]);
    }

    #[test]
    fn unclosed_cdata_runs_to_end_of_doc() {
        let mut cfg = base_config();
        cfg.per_tag_recovery
            .insert("code".into(), RecoveryStrategy::ForwardUntilTag);
        let result = parse("<code><![CDATA[if (a < b) return;]]", &cfg);
        assert_eq!(result.text, "if (a < b) return;]]");
        let code = annotated_texts(&result, "code");
        assert_eq!(code, vec!["if (a < b) return;]]"]);
    }

    #[test]
    fn autoclose_same_tag_even_when_any_disabled() {
        let mut cfg = base_config();
        cfg.autoclose_on_any_tag = false;
        cfg.autoclose_on_same_tag = true;
        let result = parse("A <note>alpha <note>beta</note>", &cfg);
        assert_eq!(result.text, "A alpha beta");
        let notes = annotated_texts(&result, "note");
        assert_eq!(notes, vec!["alpha ", "beta"]);
    }

    #[test]
    fn autoclose_same_tag_can_be_disabled() {
        let mut cfg = base_config();
        cfg.autoclose_on_any_tag = false;
        cfg.autoclose_on_same_tag = false;
        let result = parse("A <note>alpha <note>beta</note>", &cfg);
        assert_eq!(result.text, "A alpha beta");
        let notes = annotated_texts(&result, "note");
        assert_eq!(notes.join(""), "alpha beta");
    }

    #[test]
    fn autoclose_any_disabled_preserves_outer_span() {
        let mut cfg = base_config();
        cfg.autoclose_on_any_tag = false;
        cfg.autoclose_on_same_tag = false;
        let result = parse("<note>alpha <cite id=1>beta</cite>", &cfg);
        assert_eq!(result.text, "alpha beta");
        let notes = annotated_texts(&result, "note");
        assert_eq!(notes.join(""), "alpha beta");
        let cites = annotated_texts(&result, "cite");
        assert_eq!(cites, vec!["beta"]);
    }

    #[test]
    fn case_sensitive_off_allows_mixed_case_tags() {
        let mut cfg = base_config();
        cfg.case_sensitive_tags = false;
        let result = parse("<CITE id=1>Hi</CITE>", &cfg);
        let cites = annotated_texts(&result, "CITE");
        assert_eq!(cites, vec!["Hi"]);
    }

    #[test]
    fn case_sensitive_on_requires_exact_match() {
        let mut cfg = base_config();
        cfg.case_sensitive_tags = true;
        cfg.recognized_tags = ["cite"].iter().map(|s| s.to_string()).collect();
        let result = parse("<CITE id=1>Hi</CITE>", &cfg);
        assert!(annotated_texts(&result, "cite").is_empty());
        assert_eq!(result.text, "Hi");
    }

    #[test]
    fn stray_end_tag_passthrough_keeps_text() {
        let mut cfg = base_config();
        cfg.stray_end_tag_policy = StrayEndTagPolicy::Passthrough;
        let result = parse("Hello </cite>world", &cfg);
        assert_eq!(result.text, "Hello </cite>world");
    }

    #[test]
    fn retro_line_without_trim_keeps_punctuation() {
        let mut cfg = base_config();
        cfg.trim_punctuation = false;
        let result = parse("We shipped last week, <cite id=1>", &cfg);
        let cites = annotated_texts(&result, "cite");
        assert_eq!(cites, vec!["We shipped last week, "]);
    }
}
