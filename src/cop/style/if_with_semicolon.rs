use crate::cop::node_type::IF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Flags `if cond; body end` when the entire construct is on a single line.
///
/// ## Investigation (2026-03)
/// **FP root cause:** The cop was flagging multi-line `if true;\n  body\nend` where a
/// semicolon appears after the condition but the body is on the next line. This pattern
/// occurs in jruby benchmarks and other codebases (27 FPs in corpus). Prism sometimes
/// reports `;` via `then_keyword_loc`, and the fallback source-scan also found semicolons
/// before newlines. Neither path checked whether the `end` keyword was on the same line.
///
/// **Fix:** After detecting a semicolon, verify the `end` keyword is on the same line as
/// the `if` keyword. RuboCop only flags truly single-line forms (`if foo; bar end`).
pub struct IfWithSemicolon;

impl Cop for IfWithSemicolon {
    fn name(&self) -> &'static str {
        "Style/IfWithSemicolon"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Must have an `if` or `unless` keyword (not ternary)
        let if_kw_loc = match if_node.if_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        let kw_bytes = if_kw_loc.as_slice();
        if kw_bytes != b"if" && kw_bytes != b"unless" {
            return;
        }

        // Must not be modifier form (modifier has no end keyword)
        if if_node.end_keyword_loc().is_none() {
            return;
        }

        // RuboCop only flags single-line `if foo; bar end` — everything on one line.
        // Multi-line `if true;\n  body\nend` should NOT be flagged even though there
        // is a semicolon after the condition.
        //
        // Step 1: Check for semicolon between condition and body/else/end.
        // In Prism, then_keyword_loc may be ";" or "then", but Prism sometimes
        // doesn't set it. As a fallback, scan the source text.
        let has_semicolon = if let Some(then_loc) = if_node.then_keyword_loc() {
            then_loc.as_slice() == b";"
        } else {
            let pred_end = if_node.predicate().location().end_offset();
            let body_start = if let Some(stmts) = if_node.statements() {
                stmts.location().start_offset()
            } else if let Some(sub) = if_node.subsequent() {
                sub.location().start_offset()
            } else if let Some(end_loc) = if_node.end_keyword_loc() {
                end_loc.start_offset()
            } else {
                return;
            };
            if pred_end < body_start {
                let between = &source.content[pred_end..body_start];
                // Only flag if semicolon appears before any newline
                between
                    .iter()
                    .take_while(|&&b| b != b'\n')
                    .any(|&b| b == b';')
            } else {
                false
            }
        };

        if !has_semicolon {
            return;
        }

        // Step 2: Only flag if `end` is on the same line as `if` (single-line form).
        // Multi-line if with cosmetic semicolon after condition should not be flagged.
        let end_kw_loc = match if_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };
        let if_line = source.offset_to_line_col(if_kw_loc.start_offset()).0;
        let end_line = source.offset_to_line_col(end_kw_loc.start_offset()).0;
        if if_line != end_line {
            return;
        }

        let loc = if_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let cond_src =
            std::str::from_utf8(if_node.predicate().location().as_slice()).unwrap_or("...");
        let kw = std::str::from_utf8(kw_bytes).unwrap_or("if");

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Do not use `{} {};` - use a newline instead.", kw, cond_src),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IfWithSemicolon, "cops/style/if_with_semicolon");
}
