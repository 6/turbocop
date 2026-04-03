use ruby_prism::Visit;

use crate::cop::shared::node_type::STRING_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-25)
///
/// Corpus oracle reported FP=2, FN=33.
///
/// Root cause (FN=33): the source length check used byte length (`[u8]::len()`) instead of
/// Unicode character count. RuboCop uses `node.source.size.between?(2, 3)` which
/// counts characters, not bytes. Multi-byte character literals like `?中` (4 bytes
/// but 2 chars) were incorrectly skipped as meta/control characters because their
/// byte length exceeded 3. Fixed by using `str::chars().count()` instead.
///
/// ## Corpus investigation (2026-03-31)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// Root cause (FP=2): RuboCop's `StringHelp` mixin calls `ignore_node` on regexp
/// nodes (`on_regexp`), so character literals inside regexp interpolations like
/// `/#{foo.join(?,)}/` or `%r{#{bar.join(?|)}}` are skipped. Nitrocop was missing
/// this check. Fixed by detecting if the string node is inside an
/// `InterpolatedRegularExpressionNode` and skipping it.
pub struct CharacterLiteral;

impl Cop for CharacterLiteral {
    fn name(&self) -> &'static str {
        "Style/CharacterLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let string_node = match node.as_string_node() {
            Some(s) => s,
            None => return,
        };

        let opening = match string_node.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Character literals start with `?`
        if opening.as_slice() != b"?" {
            return;
        }

        // The total source of the node: ?x is 2 chars, ?\n is 3 chars
        // Allow meta and control characters like ?\C-\M-d (more than 3 chars)
        // RuboCop checks `node.source.size.between?(2, 3)` which counts
        // Unicode characters, not bytes. We must do the same for multi-byte
        // character literals like ?中 (? + 3-byte char = 4 bytes but 2 chars).
        let node_source = string_node.location().as_slice();
        let char_count = std::str::from_utf8(node_source)
            .map(|s| s.chars().count())
            .unwrap_or(node_source.len());
        if char_count > 3 {
            return;
        }

        // RuboCop's StringHelp mixin ignores string nodes inside regexp nodes
        // (on_regexp calls ignore_node, then on_str checks part_of_ignored_node?).
        // Skip character literals inside regexp interpolation like /#{foo(?,)}/.
        let loc = string_node.location();
        if is_inside_regexp(_parse_result, loc.start_offset(), loc.end_offset()) {
            return;
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Do not use the character literal - use string literal instead.".to_string(),
        );
        if let Some(ref mut corr) = corrections {
            // Replace ?x with "x" (or ?\n with "\n" etc.)
            let content = string_node.unescaped();
            let replacement = if content.len() == 1
                && content[0].is_ascii_graphic()
                && content[0] != b'\\'
                && content[0] != b'"'
            {
                format!("\"{}\"", content[0] as char)
            } else {
                // For escape sequences like ?\n, use the source text after ?
                let src = &node_source[1..];
                format!("\"{}\"", std::str::from_utf8(src).unwrap_or("?"))
            };
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
                cop_name: self.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

/// Check if the given byte range falls inside an `InterpolatedRegularExpressionNode`.
/// RuboCop's `StringHelp` mixin skips all string nodes inside regexp nodes.
fn is_inside_regexp(parse_result: &ruby_prism::ParseResult<'_>, start: usize, end: usize) -> bool {
    struct RegexpFinder {
        start: usize,
        end: usize,
        found: bool,
    }
    impl<'pr> Visit<'pr> for RegexpFinder {
        fn visit_interpolated_regular_expression_node(
            &mut self,
            node: &ruby_prism::InterpolatedRegularExpressionNode<'pr>,
        ) {
            let loc = node.location();
            if self.start >= loc.start_offset() && self.end <= loc.end_offset() {
                self.found = true;
            }
        }
    }
    let mut finder = RegexpFinder {
        start,
        end,
        found: false,
    };
    finder.visit(&parse_result.node());
    finder.found
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CharacterLiteral, "cops/style/character_literal");
    crate::cop_autocorrect_fixture_tests!(CharacterLiteral, "cops/style/character_literal");
}
