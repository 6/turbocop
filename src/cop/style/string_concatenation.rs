use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (FP=119→96→0, FN=454→0):
///
/// FP fix 1 (FP=119): Heredoc concatenation (e.g., `<<EOM + code`) — originally skipped all
/// heredoc concatenation, but RuboCop DOES fire on these (only skips autocorrect). Removed
/// the blanket heredoc skip. Heredocs are now treated as str_type? matching Parser behavior.
///
/// FP fix 2 (REVERTED): The prior percent literal exclusion was incorrect. RuboCop DOES flag
/// `config + %[...]`, `header + %{...}`, `%(str) + %(str)`, etc. All percent literal forms
/// (`%q`, `%Q`, `%()`, `%[]`, `%{}`) without interpolation are `str` in Parser, not `dstr`.
/// The blanket `%` exclusion caused ~145 FNs (percent literals not recognized as str_type?)
/// and ~9 FPs (line-end concatenation with percent literal arguments not properly detected,
/// since is_line_end_concatenation requires both sides to be str_type?). Removed the exclusion.
///
/// FP fix 3 (FP=20): Multi-line string literal concatenation. In Parser, a string literal that
/// spans multiple source lines (e.g., `'line1\nline2'` where `\n` is a real newline, not an
/// escape) is parsed as `dstr`, not `str`. So `str_type?` returns false and
/// `string_concatenation?` doesn't match. In Prism, these are still `StringNode`. Fixed by
/// checking if the StringNode source spans multiple lines and excluding those.
///
/// FN fix (FN=454): Two root causes:
/// 1. Multiline skip was too broad — skipped all multiline `str + str` regardless of where `+`
///    appeared. RuboCop only skips "line-end concatenation" where `+\s*\n` pattern exists (the `+`
///    is at the end of the line). With backslash continuation (`"str" \` + newline + `"str"`), the
///    `+` is at the start of the next line, so RuboCop flags it. Fixed by checking for `+\s*\n`.
/// 2. Dedup was inverted — skipped outer nodes when receiver was a concat chain, meaning only the
///    innermost was flagged. But inner nodes often get skipped by line-end-concat check while the
///    middle/outer nodes (with CallNode receivers, not str_type?) should still fire. Changed to
///    skip inner nodes when they're part of a larger chain (argument-side dedup).
///
/// FN/FP fix (dedup rewrite + heredoc + multiline):
/// Dedup: walk the full receiver chain to find if any inner `+` call would fire. Fire only from
/// the innermost qualifying node. Conservative mode checks leftmost part of entire chain.
/// Heredoc: removed blanket skip, heredocs now included in str_type check (matching Parser).
/// Multiline: exclude multi-line non-heredoc StringNode from str_type (they're dstr in Parser).
pub struct StringConcatenation;

impl StringConcatenation {
    /// Matches Parser's `str_type?` for a Prism node. Returns true if the node is a
    /// StringNode that would be `str` (not `dstr`) in the Parser gem.
    ///
    /// Includes: single-line quoted strings, heredocs with single-line content.
    /// Excludes: InterpolatedStringNode, percent literals, multi-line non-heredoc
    /// strings, heredocs with multi-line content (all dstr in Parser).
    fn is_str_type(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(s) = node.as_string_node() {
            if let Some(opening) = s.opening_loc() {
                let slice = opening.as_slice();
                // Heredocs (opening starts with <<):
                // In Parser, heredocs are str if content is single-line, dstr if multi-line.
                // Check the content for newlines: if content has more than one line
                // (more than one \n), it's dstr. A single trailing \n is OK (single-line).
                if slice.starts_with(b"<<") {
                    let content_bytes = s.content_loc().as_slice();
                    // Count newlines. A single-line heredoc like "content\n" has exactly 1.
                    // Multi-line like "line1\nline2\n" has 2+.
                    let newline_count = content_bytes.iter().filter(|&&b| b == b'\n').count();
                    return newline_count <= 1;
                }
            }
            // For non-heredoc strings, exclude multi-line ones (dstr in Parser).
            // Check if the node's source contains a newline.
            let loc = s.location();
            let source_bytes = loc.as_slice();
            if source_bytes.contains(&b'\n') {
                return false;
            }
            return true;
        }
        false
    }

    /// Check if this is a `+` call with exactly one argument and a receiver.
    fn is_plus_call(call: &ruby_prism::CallNode<'_>) -> bool {
        if call.name().as_slice() != b"+" {
            return false;
        }
        if let Some(args) = call.arguments() {
            let count = args.arguments().iter().count();
            return count == 1 && call.receiver().is_some();
        }
        false
    }

    /// Check if this `+` call is a string concatenation (at least one side is str_type?).
    /// Matches RuboCop's `string_concatenation?` node matcher.
    fn is_string_concat(call: &ruby_prism::CallNode<'_>) -> bool {
        if !Self::is_plus_call(call) {
            return false;
        }
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if let Some(receiver) = call.receiver() {
                return Self::is_str_type(&receiver) || Self::is_str_type(&arg_list[0]);
            }
        }
        false
    }

    /// Check if this is a line-end concatenation: both sides are simple string literals, the
    /// expression spans multiple lines, and the `+` is at the end of a line (followed
    /// by whitespace and newline). Matches RuboCop's `line_end_concatenation?` which
    /// checks `node.source.match?(/\+\s*\n/)`.
    fn is_line_end_concatenation(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> bool {
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return false,
        };
        let args = match call.arguments() {
            Some(a) => a,
            None => return false,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return false;
        }

        // Both sides must be str_type? (simple string literals or heredocs)
        if !Self::is_str_type(&receiver) || !Self::is_str_type(&arg_list[0]) {
            return false;
        }

        // Must be multiline
        let (recv_line, _) = source.offset_to_line_col(receiver.location().start_offset());
        let (arg_line, _) = source.offset_to_line_col(arg_list[0].location().start_offset());
        if recv_line == arg_line {
            return false;
        }

        // The `+` must be at the end of a line (followed by optional whitespace and newline).
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return false,
        };
        let plus_offset = msg_loc.start_offset();
        let arg_start = arg_list[0].location().start_offset();
        let src = source.as_bytes();
        if plus_offset < arg_start.min(src.len()) {
            let between = &src[plus_offset + 1..arg_start.min(src.len())];
            return between.contains(&b'\n');
        }
        false
    }

    /// Check if any `+` call in the receiver chain would independently fire
    /// (is_string_concat AND NOT line_end_concatenation).
    /// This is used for dedup: if an inner node will fire, the outer should not.
    fn has_inner_firing_node(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(receiver) = call.receiver() {
            if let Some(recv_call) = receiver.as_call_node() {
                if Self::is_plus_call(&recv_call) {
                    // Check if this receiver `+` call would fire
                    if Self::is_string_concat(&recv_call)
                        && !Self::is_line_end_concatenation(source, &recv_call)
                    {
                        return true;
                    }
                    // Recurse: check deeper in the chain
                    return Self::has_inner_firing_node(source, &recv_call);
                }
            }
        }
        false
    }

    /// Find the leftmost (deepest) non-`+` part of the chain. Used for conservative mode.
    fn leftmost_part<'a>(call: &ruby_prism::CallNode<'a>) -> Option<ruby_prism::Node<'a>> {
        if let Some(receiver) = call.receiver() {
            if let Some(recv_call) = receiver.as_call_node() {
                if Self::is_plus_call(&recv_call) {
                    return Self::leftmost_part(&recv_call);
                }
            }
            return Some(receiver);
        }
        None
    }
}

impl Cop for StringConcatenation {
    fn name(&self) -> &'static str {
        "Style/StringConcatenation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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

        if !Self::is_string_concat(&call) {
            return;
        }

        // Skip line-end concatenation where both sides are str_type?, the
        // expression spans multiple lines, and the `+` is at the end of a line.
        // This is handled by Style/LineEndConcatenation instead.
        if Self::is_line_end_concatenation(source, &call) {
            return;
        }

        // Dedup chains: if any inner `+` call in the receiver chain would
        // independently fire (is_string_concat, not line-end-concat),
        // skip this node. The inner one will fire at the same start position.
        // This matches RuboCop's behavior of reporting one offense per chain.
        if Self::has_inner_firing_node(source, &call) {
            return;
        }

        // Conservative mode: check if the leftmost part of the entire chain is
        // str_type?. RuboCop walks up to the topmost `+` node, collects all
        // parts, and checks `parts.first.str_type?`.
        let mode = config.get_str("Mode", "aggressive");
        if mode == "conservative" {
            if let Some(leftmost) = Self::leftmost_part(&call) {
                if !Self::is_str_type(&leftmost) {
                    return;
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Prefer string interpolation to string concatenation.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringConcatenation, "cops/style/string_concatenation");
}
