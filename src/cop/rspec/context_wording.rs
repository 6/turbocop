use crate::cop::shared::constant_predicates;
use crate::cop::shared::node_type::{
    CALL_NODE, INTERPOLATED_STRING_NODE, INTERPOLATED_X_STRING_NODE, STRING_NODE, X_STRING_NODE,
};
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// FP=17, FN=112. Root cause of FPs: nitrocop was checking CallNode directly
/// without requiring a block. RuboCop uses `on_block` with pattern
/// `(block (send #rspec? { :context :shared_context } ...) ...)` — only fires
/// when the call has a `do...end` or `{ }` block. Also missing receiver check:
/// RuboCop requires nil receiver or `RSpec` constant.
///
/// ## Corpus investigation (2026-03-11)
///
/// FP=5: Word boundary mismatch. nitrocop used `strip_prefix` then checked if
/// remainder starts with space/comma/newline. RuboCop uses `\b` regex word
/// boundary. Descriptions like "when-something" or "with.dots" should NOT be
/// flagged because `-` and `.` are non-word chars that satisfy `\b`.
/// Fix: check that the char after the prefix is absent or not `[a-zA-Z0-9_]`.
///
/// FN=112: Missing xstr (backtick string) handling. RuboCop's `any_str` pattern
/// matches `str`, `dstr`, AND `xstr`. nitrocop only handled StringNode and
/// InterpolatedStringNode, missing XStringNode and InterpolatedXStringNode.
/// Fix: add xstr node types to interested_node_types and extract their content.
///
/// ## Corpus investigation (2026-03-14)
///
/// FP=2, FN=95.
///
/// FP=2: asciidoctor__asciidoctor-pdf. Both FPs are `context 'Cache', if: ...,
/// &(proc do ... end)` and `context 'ICU', if: ..., &(proc do ... end)` style.
/// Root cause: `&(proc do end)` stores a `BlockArgumentNode` in `call.block()`,
/// not a `BlockNode`. RuboCop's `on_block` pattern only fires for `BlockNode`.
/// Fix: require `call.block().as_block_node().is_some()` instead of `is_some()`.
///
/// ## Corpus investigation (2026-03-15)
///
/// FN=112 (was FN=95): Interpolated strings starting with an interpolation
/// (e.g., `"#{var} elements"`) were skipped entirely because
/// `extract_interp_leading_text` returned `None` when the first part was
/// an `EmbeddedStatementsNode` rather than a `StringNode`. RuboCop reports
/// an offense because the description can't match any prefix.
/// Fix: return empty string instead of None when no leading text exists,
/// so the prefix check correctly fails and reports the offense. Same fix
/// applied to interpolated xstr nodes.
///
/// ## Corpus investigation (2026-03-18)
///
/// FP=1: rubocop__rubocop. Implicit string concatenation with line continuation
/// (`"when #{var}: " \ 'extra text'`) creates a nested `InterpolatedStringNode`
/// inside the outer `InterpolatedStringNode`. `extract_interp_leading_text` only
/// checked for `StringNode` as the first part, missing the nested interp case.
/// Fix: recurse into nested `InterpolatedStringNode` to extract leading text.
pub struct ContextWording;

const DEFAULT_PREFIXES: &[&str] = &["when", "with", "without"];

impl Cop for ContextWording {
    fn name(&self) -> &'static str {
        "RSpec/ContextWording"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            INTERPOLATED_STRING_NODE,
            INTERPOLATED_X_STRING_NODE,
            STRING_NODE,
            X_STRING_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();
        if method != b"context" && method != b"shared_context" {
            return;
        }

        // RuboCop uses on_block: requires an actual BlockNode (do...end or { }).
        // &(proc do end) stores a BlockArgumentNode, not a BlockNode — skip it.
        if call.block().is_none_or(|b| b.as_block_node().is_none()) {
            return;
        }

        // Receiver must be nil or RSpec constant
        if let Some(recv) = call.receiver() {
            if constant_predicates::constant_short_name(&recv).is_none_or(|n| n != b"RSpec") {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Extract description text from string, interpolated string, or xstr (backtick)
        // RuboCop's `any_str` matches str, dstr, and xstr node types.
        let content_str: String;
        if let Some(s) = arg_list[0].as_string_node() {
            let content = s.unescaped();
            content_str = match std::str::from_utf8(content) {
                Ok(s) => s.to_string(),
                Err(_) => return,
            };
        } else if let Some(interp) = arg_list[0].as_interpolated_string_node() {
            // For interpolated strings, extract leading text before first interpolation.
            // Returns empty string if string starts with interpolation (no prefix can match).
            content_str = extract_interp_leading_text(&interp);
        } else if let Some(x) = arg_list[0].as_x_string_node() {
            let content = x.unescaped();
            content_str = match std::str::from_utf8(content) {
                Ok(s) => s.to_string(),
                Err(_) => return,
            };
        } else if let Some(interp_x) = arg_list[0].as_interpolated_x_string_node() {
            // For interpolated xstr, extract leading text before first interpolation.
            // Returns empty string if string starts with interpolation (no prefix can match).
            let parts: Vec<_> = interp_x.parts().iter().collect();
            content_str = if let Some(first) = parts.first() {
                if let Some(s) = first.as_string_node() {
                    let text = s.unescaped();
                    std::str::from_utf8(text)
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
        } else {
            return;
        };

        // Config: AllowedPatterns — regex patterns to skip
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        // Check if description matches any allowed pattern
        if let Some(ref patterns) = allowed_patterns {
            for pat in patterns {
                if let Ok(re) = regex::Regex::new(pat) {
                    if re.is_match(&content_str) {
                        return;
                    }
                }
            }
        }

        // Read Prefixes from config, fall back to defaults
        let config_prefixes = config.get_string_array("Prefixes");
        let prefixes: Vec<&str> = if let Some(ref arr) = config_prefixes {
            arr.iter().map(|s| s.as_str()).collect()
        } else {
            DEFAULT_PREFIXES.to_vec()
        };

        // Check if description starts with any allowed prefix followed by a word boundary.
        // RuboCop uses /^#{Regexp.escape(pre)}\b/ which matches when the next char
        // is not a word character [a-zA-Z0-9_]. This means "when-foo" matches (dash
        // is non-word), but "whenever" does not (e is a word char).
        for prefix in &prefixes {
            if let Some(after) = content_str.strip_prefix(prefix) {
                if after.is_empty() {
                    return;
                }
                let next_is_word_char =
                    after.as_bytes()[0].is_ascii_alphanumeric() || after.as_bytes()[0] == b'_';
                if !next_is_word_char {
                    return;
                }
            }
        }

        let prefix_display: Vec<String> = prefixes.iter().map(|p| format!("/^{p}\\b/")).collect();
        let loc = arg_list[0].location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Context description should match {}.",
                prefix_display.join(", ")
            ),
        ));
    }
}

/// Extract leading text from an interpolated string's parts (before first interpolation).
/// Returns empty string if the string starts with an interpolation (no leading text).
///
/// Handles implicit string concatenation (e.g., `"when #{var}: " 'extra'`) where
/// Prism wraps segments in a parent InterpolatedStringNode whose first part is itself
/// an InterpolatedStringNode. In that case, recurse into the nested node.
fn extract_interp_leading_text(interp: &ruby_prism::InterpolatedStringNode<'_>) -> String {
    let parts: Vec<_> = interp.parts().iter().collect();
    let Some(first) = parts.first() else {
        return String::new();
    };
    if let Some(s) = first.as_string_node() {
        let text = s.unescaped();
        return std::str::from_utf8(text)
            .map(|s| s.to_string())
            .unwrap_or_default();
    }
    // Implicit string concatenation: first part may be a nested InterpolatedStringNode.
    // E.g., `"when #{var}: " 'text'` → outer InterpolatedStringNode with parts:
    //   [InterpolatedStringNode("when #{var}: "), StringNode("text")]
    // Recurse into the nested interpolated string to extract its leading text.
    if let Some(nested_interp) = first.as_interpolated_string_node() {
        return extract_interp_leading_text(&nested_interp);
    }
    // First part is an interpolation (EmbeddedStatementsNode), not text.
    // The description starts with a dynamic value, so no prefix can match.
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ContextWording, "cops/rspec/context_wording");

    #[test]
    fn allowed_patterns_skips_matching_description() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("^if ".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"context 'if the user is logged in' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ContextWording, source, config);
        assert!(
            diags.is_empty(),
            "AllowedPatterns should skip matching descriptions"
        );
    }

    #[test]
    fn implicit_concat_with_interpolation_not_flagged() {
        // Ruby: context "when the value includes #{var}: " \
        //               'extra text' do
        // end
        // Prism parses this as an outer InterpolatedStringNode wrapping a nested
        // InterpolatedStringNode and a StringNode. The leading text is "when ..."
        // which matches the "when" prefix, so it should NOT be flagged.
        let src =
            b"context \"when the value includes \x23{var}: \" \\\n        'extra text' do\nend\n";
        let diags = crate::testutil::run_cop_full(&ContextWording, src);
        assert!(
            diags.is_empty(),
            "Should not flag 'when...' with line continuation concat: got {} diagnostics",
            diags.len()
        );
    }

    #[test]
    fn allowed_patterns_does_not_skip_non_matching() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("^if ".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"context 'the user is logged in' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ContextWording, source, config);
        assert_eq!(diags.len(), 1);
    }
}
