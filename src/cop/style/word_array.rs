use crate::cop::node_type::{ARRAY_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Default WordRegex pattern matching RuboCop's default:
/// `/\A(?:\p{Word}|\p{Word}-\p{Word}|\n|\t)+\z/`
/// Translated to Rust regex syntax: \A → ^, \z → $, \p{Word} → \w
const DEFAULT_WORD_REGEX: &str = r"^(?:\w|\w-\w|\n|\t)+$";

pub struct WordArray;

/// Extract a Ruby regexp pattern from a string like `/pattern/flags`.
/// Returns the inner pattern without delimiters and flags.
fn extract_word_regex(s: &str) -> Option<&str> {
    let s = s.trim();
    if s.starts_with('/') && s.len() > 1 {
        if let Some(end) = s[1..].rfind('/') {
            return Some(&s[1..end + 1]);
        }
    }
    None
}

/// Translate Ruby regex syntax to Rust regex syntax.
fn translate_ruby_regex(pattern: &str) -> String {
    pattern
        .replace(r"\A", "^")
        .replace(r"\z", "$")
        .replace(r"\p{Word}", r"\w")
}

/// Build a compiled regex from the WordRegex config value.
/// Falls back to the default pattern if the config value is empty or unparseable.
fn build_word_regex(config_value: &str) -> Option<regex::Regex> {
    if config_value.is_empty() {
        return regex::Regex::new(DEFAULT_WORD_REGEX).ok();
    }
    let raw_pattern = if let Some(inner) = extract_word_regex(config_value) {
        inner
    } else {
        config_value
    };
    let translated = translate_ruby_regex(raw_pattern);
    regex::Regex::new(&translated).ok()
}

impl Cop for WordArray {
    fn name(&self) -> &'static str {
        "Style/WordArray"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        // Must have `[` opening (not %w or %W)
        let opening = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        if opening.as_slice() != b"[" {
            return;
        }

        let elements = array_node.elements();
        let min_size = config.get_usize("MinSize", 2);
        let enforced_style = config.get_str("EnforcedStyle", "percent");
        let word_regex_str = config.get_str("WordRegex", "");

        // "brackets" style: never flag bracket arrays
        if enforced_style == "brackets" {
            return;
        }

        if elements.len() < min_size {
            return;
        }

        // Skip arrays that contain comments — converting to %w would lose them
        let array_start = opening.start_offset();
        let array_end = array_node
            .closing_loc()
            .map(|c| c.end_offset())
            .unwrap_or(array_start);
        if has_comment_in_range(parse_result, array_start, array_end) {
            return;
        }

        // Build compiled word regex (default handles hyphens, unicode, \n, \t)
        let word_re = build_word_regex(word_regex_str);

        // All elements must be simple string nodes with word-like content
        for elem in elements.iter() {
            let string_node = match elem.as_string_node() {
                Some(s) => s,
                None => return,
            };

            // Must have an opening quote (not a bare string)
            if string_node.opening_loc().is_none() {
                return;
            }

            // Use unescaped content (interpreted value, like RuboCop's str_content)
            let unescaped_bytes = string_node.unescaped();

            // Content must not be empty (empty strings can't be in %w)
            if unescaped_bytes.is_empty() {
                return;
            }

            // Content must not contain spaces
            if unescaped_bytes.contains(&b' ') {
                return;
            }

            // Check content against WordRegex
            let content_str = match std::str::from_utf8(unescaped_bytes) {
                Ok(s) => s,
                Err(_) => return, // Invalid UTF-8 → complex content
            };

            if let Some(ref re) = word_re {
                if !re.is_match(content_str) {
                    return;
                }
            }
        }

        let (line, column) = source.offset_to_line_col(opening.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `%w` or `%W` for an array of words.".to_string(),
        ));
    }
}

/// Check if there are any comments within a byte offset range.
fn has_comment_in_range(
    parse_result: &ruby_prism::ParseResult<'_>,
    start: usize,
    end: usize,
) -> bool {
    for comment in parse_result.comments() {
        let comment_start = comment.location().start_offset();
        if comment_start >= start && comment_start < end {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(WordArray, "cops/style/word_array");

    #[test]
    fn config_min_size_5() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("MinSize".into(), serde_yml::Value::Number(5.into()))]),
            ..CopConfig::default()
        };
        // 5 elements should trigger with MinSize:5
        let source = b"x = ['a', 'b', 'c', 'd', 'e']\n";
        let diags = run_cop_full_with_config(&WordArray, source, config.clone());
        assert!(
            !diags.is_empty(),
            "Should fire with MinSize:5 on 5-element word array"
        );

        // 4 elements should NOT trigger
        let source2 = b"x = ['a', 'b', 'c', 'd']\n";
        let diags2 = run_cop_full_with_config(&WordArray, source2, config);
        assert!(
            diags2.is_empty(),
            "Should not fire on 4-element word array with MinSize:5"
        );
    }

    #[test]
    fn default_word_regex_rejects_hyphens_only() {
        let re = build_word_regex("").unwrap();
        assert!(!re.is_match("-"), "single hyphen should not match");
        assert!(!re.is_match("----"), "multiple hyphens should not match");
        assert!(re.is_match("foo"), "simple word should match");
        assert!(re.is_match("foo-bar"), "hyphenated word should match");
        assert!(re.is_match("one\n"), "word with newline should match");
        assert!(!re.is_match(" "), "space should not match");
        assert!(!re.is_match(""), "empty should not match");
    }

    #[test]
    fn brackets_style_allows_bracket_arrays() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("brackets".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"x = ['a', 'b', 'c']\n";
        let diags = run_cop_full_with_config(&WordArray, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag brackets with brackets style"
        );
    }
}
