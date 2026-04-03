use crate::cop::shared::node_type::IN_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/InPatternThen: flags `in pattern; body` and suggests `in pattern then body`.
///
/// ## Investigation (2026-03-10)
/// FP=8, FN=0. Three root causes:
/// 1. rufo (4 FPs): `in 2; then puts "2"` — semicolon AND `then` keyword both present.
///    RuboCop checks `node.then?` (presence of `then` keyword) and skips. Fix: check if
///    the source between pattern and body contains `then` after the `;`.
/// 2. jruby (3 FPs): `in 0, 1,;` / `in *;` / `in **;` — multiline patterns where the
///    body is on the next line. RuboCop checks `node.multiline?` and skips. Fix: check
///    if the `in` node spans multiple lines; only flag single-line nodes.
/// 3. danbooru (1 FP): multiline `in` pattern with regex. Same multiline fix.
pub struct InPatternThen;

impl Cop for InPatternThen {
    fn name(&self) -> &'static str {
        "Style/InPatternThen"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IN_NODE]
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
        let in_node = match node.as_in_node() {
            Some(n) => n,
            None => return,
        };

        let pattern = in_node.pattern();
        let pattern_end = pattern.location().end_offset();

        let src = source.as_bytes();
        if let Some(stmts) = in_node.statements() {
            let stmts_start = stmts.location().start_offset();
            let between = &src[pattern_end..stmts_start];
            if let Some(pos) = between.iter().position(|&b| b == b';') {
                let semi_offset = pattern_end + pos;

                // RuboCop: `return if node.multiline?` — skip if the `in` node
                // spans multiple lines (e.g., `in *;\n  true`).
                let in_start_line = source.offset_to_line_col(node.location().start_offset()).0;
                let in_end_line = source
                    .offset_to_line_col(
                        node.location().start_offset() + node.location().as_slice().len(),
                    )
                    .0;
                if in_start_line != in_end_line {
                    return;
                }

                // RuboCop: `return if node.then?` — skip if a `then` keyword is
                // also present (e.g., `in 2; then puts "2"`).
                let after_semi = &between[pos + 1..];
                if after_semi.windows(4).any(|w| w == b"then") {
                    return;
                }

                let (line, column) = source.offset_to_line_col(semi_offset);
                let pattern_src = String::from_utf8_lossy(pattern.location().as_slice());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Do not use `in {}`. Use `in {} then` instead.",
                        pattern_src, pattern_src
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InPatternThen, "cops/style/in_pattern_then");
}
