use std::collections::{HashMap, HashSet};

use crate::types::{
    Annotation, AttrValue, Marker, ParseResult, ParserConfig, RecoveryStrategy, Segment,
    StrayEndTagPolicy, UnknownMode,
};

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
        while idx < self.input.len() {
            let remaining = &self.input[idx..];

            if remaining.starts_with("<![CDATA[") {
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

            if remaining.starts_with('<') {
                if let Some((token, consumed)) = self.parse_tag(idx) {
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
                } else {
                    // Treat a lone '<' as literal text and advance by one character.
                    let mut buf = [0u8; 4];
                    let ch = remaining.chars().next().unwrap();
                    let as_str = ch.encode_utf8(&mut buf);
                    self.push_text(as_str);
                    idx += ch.len_utf8();
                    continue;
                }
            }

            if let Some(next_lt) = remaining.find('<') {
                let slice = &remaining[..next_lt];
                self.push_text(slice);
                idx += slice.len();
            } else {
                self.push_text(remaining);
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
