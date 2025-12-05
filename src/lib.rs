//! KindaXML: close-enough, XML-ish annotations for LLM output.
//!
//! Basic usage:
//!
//! ```
//! use kindaxml::{parse, ParserConfig};
//!
//! let mut cfg = ParserConfig::default();
//! cfg.recognized_tags = ["cite"].into_iter().map(String::from).collect();
//! cfg.case_sensitive_tags = false;
//!
//! let parsed = parse("We shipped <cite id=1>last week</cite>.", &cfg);
//! assert_eq!(parsed.text, "We shipped last week.");
//! assert_eq!(parsed.segments[1].annotations[0].tag, "cite");
//! ```
//!
//! See `ParserConfig` for knobs that control recovery and unknown tag handling.

pub mod parser;
pub mod types;

#[cfg(feature = "python")]
mod python_bindings;

pub use parser::parse;
pub use types::*;

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
        let recognized_tags = ["cite", "note", "todo", "claim", "risk", "code"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut cfg = ParserConfig {
            recognized_tags,
            trim_punctuation: true,
            case_sensitive_tags: false,
            ..ParserConfig::default()
        };
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
