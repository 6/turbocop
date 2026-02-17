use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct IfWithSemicolon;

impl Cop for IfWithSemicolon {
    fn name(&self) -> &'static str {
        "Style/IfWithSemicolon"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Must have an `if` or `unless` keyword (not ternary)
        let if_kw_loc = match if_node.if_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let kw_bytes = if_kw_loc.as_slice();
        if kw_bytes != b"if" && kw_bytes != b"unless" {
            return Vec::new();
        }

        // Must not be modifier form (modifier has no end keyword)
        if if_node.end_keyword_loc().is_none() {
            return Vec::new();
        }

        // Check if there's a semicolon between the predicate and the body/else/end.
        // Prism may or may not set then_keyword_loc for semicolons.
        let has_semicolon = if let Some(then_loc) = if_node.then_keyword_loc() {
            then_loc.as_slice() == b";"
        } else {
            // Check the source between predicate end and body/end start
            let pred_end = if_node.predicate().location().end_offset();
            let body_start = if let Some(stmts) = if_node.statements() {
                stmts.location().start_offset()
            } else if let Some(sub) = if_node.subsequent() {
                sub.location().start_offset()
            } else if let Some(end_loc) = if_node.end_keyword_loc() {
                end_loc.start_offset()
            } else {
                return Vec::new();
            };
            if pred_end < body_start {
                let between = &source.content[pred_end..body_start];
                between.contains(&b';')
            } else {
                false
            }
        };

        if !has_semicolon {
            return Vec::new();
        }

        let loc = if_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let cond_src = std::str::from_utf8(if_node.predicate().location().as_slice()).unwrap_or("...");
        let kw = std::str::from_utf8(kw_bytes).unwrap_or("if");

        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Do not use `{} {};` - use a newline instead.", kw, cond_src),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IfWithSemicolon, "cops/style/if_with_semicolon");
}
