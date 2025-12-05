use std::collections::{HashMap, HashSet};

/// A single attribute value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValue {
    Bool(bool),
    Str(String),
}

/// An annotation applied to a span of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    pub tag: String,
    pub attrs: HashMap<String, AttrValue>,
}

/// A contiguous run of text plus any annotations covering it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub annotations: Vec<Annotation>,
}

/// A zero-width marker produced by self-closing tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    pub pos: usize,
    pub annotation: Annotation,
}

/// How to treat unknown tags encountered during parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownMode {
    /// Drop the unknown tag markup but keep inner text.
    Strip,
    /// Keep the literal `<unknown ...>` markup in the output text.
    Passthrough,
    /// Do not treat `<...>` as tags at all; parse them as plain text.
    TreatAsText,
}

/// Recovery strategies for unclosed tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Retroactively apply the tag to the text on the same line before the tag.
    RetroLine,
    /// Apply the tag forward until the next tag start.
    ForwardUntilTag,
    /// Apply the tag forward until the next newline.
    ForwardUntilNewline,
    /// Apply the tag to the next token/word only.
    ForwardNextToken,
    /// Ignore the unclosed tag (no annotation emitted).
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrayEndTagPolicy {
    /// Drop stray end tags for recognized tags.
    Drop,
    /// Keep stray end tags as literal text.
    Passthrough,
}

/// Parser configuration controlling tag recognition and recovery.
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Whitelist of tags that should be parsed/annotated.
    pub recognized_tags: HashSet<String>,
    /// Per-tag recovery overrides for unclosed tags.
    pub per_tag_recovery: HashMap<String, RecoveryStrategy>,
    /// How to handle unknown tags.
    pub unknown_mode: UnknownMode,
    /// If true, close open tags when encountering any new tag.
    pub autoclose_on_any_tag: bool,
    /// If true, close open tags when seeing the same tag again.
    pub autoclose_on_same_tag: bool,
    /// Trim punctuation/whitespace when retroactively selecting spans.
    pub trim_punctuation: bool,
    /// Match tags with case sensitivity.
    pub case_sensitive_tags: bool,
    /// What to do with stray end tags (no matching open).
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

impl ParserConfig {
    /// Default parser config tuned for LLM-style tags.
    pub fn default_llm_friendly_config() -> ParserConfig {
        let recognized_tags: HashSet<String> = ["cite", "note", "todo", "claim", "risk", "code"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut per_tag_recovery: HashMap<String, RecoveryStrategy> = HashMap::new();
        per_tag_recovery.insert("cite".into(), RecoveryStrategy::RetroLine);
        for tag in ["note", "todo", "claim", "risk", "code"] {
            per_tag_recovery.insert(tag.into(), RecoveryStrategy::ForwardUntilTag);
        }
        ParserConfig {
            recognized_tags,
            per_tag_recovery,
            trim_punctuation: true,
            case_sensitive_tags: false,
            ..ParserConfig::default()
        }
    }

    /// Default LLM cite parser configuration.
    pub fn default_cite_config() -> ParserConfig {
        ParserConfig {
            recognized_tags: ["cite"].iter().map(|s| s.to_string()).collect(),
            per_tag_recovery: [("cite".into(), RecoveryStrategy::RetroLine)]
                .iter()
                .cloned()
                .collect(),
            trim_punctuation: true,
            case_sensitive_tags: false,
            ..ParserConfig::default()
        }
    }
}

/// Parser output: plain text, spans, and markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    pub text: String,
    pub segments: Vec<Segment>,
    pub markers: Vec<Marker>,
}
