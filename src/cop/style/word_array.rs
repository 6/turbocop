use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Default WordRegex pattern matching RuboCop's default:
/// `/\A(?:\p{Word}|\p{Word}-\p{Word}|\n|\t)+\z/`
/// Translated to Rust regex syntax: \A → ^, \z → $, \p{Word} → \w
const DEFAULT_WORD_REGEX: &str = r"^(?:\w|\w-\w|\n|\t)+$";

/// Style/WordArray: flags bracket arrays of word-like strings that could use %w.
///
/// ## Investigation findings
///
/// **FP fix 1:** Missing `within_matrix_of_complex_content?` check from RuboCop.
/// When a bracket array is nested inside a parent array (a "matrix") where ALL
/// elements are arrays, and at least ONE sibling subarray has complex content
/// (strings with spaces, non-word characters, or invalid encoding), RuboCop
/// exempts the entire matrix.
///
/// **FP fix 2:** Missing `invalid_percent_array_context?` check from RuboCop's
/// PercentArray mixin. When a bracket word array is an argument to a
/// non-parenthesized method call that also has a block (e.g.,
/// `describe_pattern "LOG", ['legacy', 'ecs-v1'] do ... end`), `%w()`
/// would be ambiguous — Ruby cannot distinguish `{` as a block vs hash
/// literal. RuboCop exempts these arrays and so must we. (FP=107 fixed)
///
/// **FN fix:** The `is_matrix_of_complex_content` function was incorrectly
/// treating non-string elements (integers, symbols) as "complex content".
/// RuboCop's `complex_content?` method skips non-string elements (`next unless
/// s.str_content`). This caused parent arrays with mixed-type subarrays
/// (e.g., `[["foo", "bar", 0], ["baz", "qux"]]`) to be wrongly classified as
/// complex-content matrices, suppressing pure-string subarrays. (FN=314 fixed)
///
/// **Remaining FN (158):** Primarily `brackets` style enforcement direction
/// (flagging `%w[...]` arrays for conversion to brackets), which is not yet
/// implemented, plus some edge cases in deeply nested structures.
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

/// Check if an array node has complex content (any string element that doesn't
/// match the word regex, contains spaces, is empty, or has invalid encoding).
fn array_has_complex_content(
    array_node: &ruby_prism::ArrayNode<'_>,
    word_re: &Option<regex::Regex>,
) -> bool {
    for elem in array_node.elements().iter() {
        let string_node = match elem.as_string_node() {
            Some(s) => s,
            None => return true, // non-string element = complex
        };
        if string_node.opening_loc().is_none() {
            return true;
        }
        let unescaped_bytes = string_node.unescaped();
        if unescaped_bytes.is_empty() {
            return true;
        }
        if unescaped_bytes.contains(&b' ') {
            return true;
        }
        let content_str = match std::str::from_utf8(unescaped_bytes) {
            Ok(s) => s,
            Err(_) => return true,
        };
        if let Some(re) = word_re {
            if !re.is_match(content_str) {
                return true;
            }
        }
    }
    false
}

/// Check if a subarray has complex string content, matching RuboCop's
/// `complex_content?` semantics. Non-string elements are SKIPPED (not treated
/// as complex). Only string elements with spaces, empty content, invalid
/// encoding, or non-word characters count as complex.
fn subarray_has_complex_string_content(
    array_node: &ruby_prism::ArrayNode<'_>,
    word_re: &Option<regex::Regex>,
) -> bool {
    for elem in array_node.elements().iter() {
        let string_node = match elem.as_string_node() {
            Some(s) => s,
            None => continue, // skip non-string elements (RuboCop: `next unless s.str_content`)
        };
        if string_node.opening_loc().is_none() {
            continue;
        }
        let unescaped_bytes = string_node.unescaped();
        // Empty strings and strings with spaces are complex
        if unescaped_bytes.is_empty() || unescaped_bytes.contains(&b' ') {
            return true;
        }
        let content_str = match std::str::from_utf8(unescaped_bytes) {
            Ok(s) => s,
            Err(_) => return true,
        };
        if let Some(re) = word_re {
            if !re.is_match(content_str) {
                return true;
            }
        }
    }
    false
}

/// Check if a parent array is a "matrix of complex content": all elements are
/// arrays, and at least one has complex string content. Matches RuboCop's
/// `matrix_of_complex_content?` method.
fn is_matrix_of_complex_content(
    array_node: &ruby_prism::ArrayNode<'_>,
    word_re: &Option<regex::Regex>,
) -> bool {
    let elements = array_node.elements();
    if elements.is_empty() {
        return false;
    }
    let mut any_complex = false;
    for elem in elements.iter() {
        let sub = match elem.as_array_node() {
            Some(a) => a,
            None => return false, // not all elements are arrays
        };
        if !any_complex && subarray_has_complex_string_content(&sub, word_re) {
            any_complex = true;
        }
    }
    any_complex
}

impl Cop for WordArray {
    fn name(&self) -> &'static str {
        "Style/WordArray"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let min_size = config.get_usize("MinSize", 2);
        let enforced_style = config.get_str("EnforcedStyle", "percent");
        let word_regex_str = config.get_str("WordRegex", "");

        if enforced_style == "brackets" {
            return;
        }

        let word_re = build_word_regex(word_regex_str);

        let mut visitor = WordArrayVisitor {
            cop: self,
            source,
            parse_result,
            min_size,
            word_re,
            in_matrix_of_complex_content: false,
            in_ambiguous_block_context: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct WordArrayVisitor<'a, 'src, 'pr> {
    cop: &'a WordArray,
    source: &'src SourceFile,
    parse_result: &'a ruby_prism::ParseResult<'pr>,
    min_size: usize,
    word_re: Option<regex::Regex>,
    in_matrix_of_complex_content: bool,
    /// True when inside direct arguments of a non-parenthesized call with a block.
    in_ambiguous_block_context: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> WordArrayVisitor<'_, '_, 'pr> {
    fn check_array(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        // Must have `[` opening (not %w or %W)
        let opening = match node.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        if opening.as_slice() != b"[" {
            return;
        }

        let elements = node.elements();

        if elements.len() < self.min_size {
            return;
        }

        // Skip if inside a matrix of complex content
        if self.in_matrix_of_complex_content {
            return;
        }

        // Skip if in ambiguous block context (invalid_percent_array_context?)
        if self.in_ambiguous_block_context {
            return;
        }

        // Skip arrays that contain comments
        let array_start = opening.start_offset();
        let array_end = node
            .closing_loc()
            .map(|c| c.end_offset())
            .unwrap_or(array_start);
        if has_comment_in_range(self.parse_result, array_start, array_end) {
            return;
        }

        // All elements must be simple string nodes with word-like content
        if array_has_complex_content(node, &self.word_re) {
            return;
        }

        let (line, column) = self.source.offset_to_line_col(opening.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `%w` or `%W` for an array of words.".to_string(),
        ));
    }

    /// Check if a call node represents an ambiguous block context:
    /// non-parenthesized method call with a block.
    fn is_ambiguous_block_call(&self, call: &ruby_prism::CallNode<'pr>) -> bool {
        // Must have a block
        if call.block().is_none() {
            return false;
        }
        // Must have arguments
        if call.arguments().is_none() {
            return false;
        }
        // Must NOT be parenthesized
        call.opening_loc().is_none()
    }
}

impl<'pr> Visit<'pr> for WordArrayVisitor<'_, '_, 'pr> {
    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        // Check if this array is a matrix of complex content before visiting children
        let is_matrix = is_matrix_of_complex_content(node, &self.word_re);
        let prev = self.in_matrix_of_complex_content;
        if is_matrix {
            self.in_matrix_of_complex_content = true;
        }

        self.check_array(node);

        // Visit children to check nested arrays
        ruby_prism::visit_array_node(self, node);

        self.in_matrix_of_complex_content = prev;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.is_ambiguous_block_call(node) {
            // Visit receiver normally
            if let Some(receiver) = node.receiver() {
                self.visit(&receiver);
            }
            // Visit arguments — only suppress top-level ArrayNode arguments,
            // matching RuboCop's `parent.arguments.include?(node)` check.
            if let Some(args) = node.arguments() {
                let prev = self.in_ambiguous_block_context;
                for arg in args.arguments().iter() {
                    if arg.as_array_node().is_some() {
                        self.in_ambiguous_block_context = true;
                        self.visit(&arg);
                        self.in_ambiguous_block_context = prev;
                    } else {
                        self.visit(&arg);
                    }
                }
            }
            // Visit block normally — arrays inside block body are NOT ambiguous
            if let Some(block) = node.block() {
                self.visit(&block);
            }
        } else {
            ruby_prism::visit_call_node(self, node);
        }
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

    #[test]
    fn matrix_with_mixed_types_does_not_suppress_pure_string_subarrays() {
        use crate::testutil::run_cop_full;

        // Parent array where some subarrays mix strings and integers.
        // RuboCop's complex_content? skips non-strings, so the matrix
        // is NOT considered "complex content". Pure-string subarrays
        // like ["foo", "bar"] should still be flagged.
        let source = b"x = [[\"foo\", \"bar\", 0], [\"baz\", \"qux\"]]\n";
        let diags = run_cop_full(&WordArray, source);
        assert!(
            !diags.is_empty(),
            "Should flag pure-string subarrays in a matrix with mixed-type siblings"
        );
    }

    #[test]
    fn all_word_matrix_flags_subarrays() {
        use crate::testutil::run_cop_full;

        // Matrix where all subarrays are simple word arrays.
        // RuboCop flags each subarray since there's no complex content.
        let source = b"[['Architecture', 'view_architectures'], ['Audit', 'view_audit_logs']]\n";
        let diags = run_cop_full(&WordArray, source);
        assert_eq!(
            diags.len(),
            2,
            "Should flag both subarrays in an all-word matrix"
        );
    }
}
