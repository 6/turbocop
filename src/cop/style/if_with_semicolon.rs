use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::IF_NODE;

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

        // RuboCop checks node.loc.begin which is the then keyword.
        // In Prism, then_keyword_loc may be ";" or "then", but Prism sometimes
        // doesn't set it for single-line `if foo; bar end`. As a fallback, scan
        // the source between predicate end and body start on the SAME LINE only
        // (to avoid false positives from semicolons in comments on subsequent lines).
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
                // (single-line if with semicolon vs multi-line if with comments)
                between.iter().take_while(|&&b| b != b'\n').any(|&b| b == b';')
            } else {
                false
            }
        };

        if !has_semicolon {
            return;
        }

        let loc = if_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let cond_src = std::str::from_utf8(if_node.predicate().location().as_slice()).unwrap_or("...");
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
