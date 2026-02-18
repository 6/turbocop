use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct WhileUntilModifier;

impl Cop for WhileUntilModifier {
    fn name(&self) -> &'static str {
        "Style/WhileUntilModifier"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let (kw_loc, statements, keyword) = if let Some(while_node) = node.as_while_node() {
            (while_node.keyword_loc(), while_node.statements(), "while")
        } else if let Some(until_node) = node.as_until_node() {
            (until_node.keyword_loc(), until_node.statements(), "until")
        } else {
            return Vec::new();
        };

        // Skip modifier form — check if keyword comes before the closing
        let closing_loc = if let Some(while_node) = node.as_while_node() {
            while_node.closing_loc()
        } else if let Some(until_node) = node.as_until_node() {
            until_node.closing_loc()
        } else {
            return Vec::new();
        };

        // If no closing (end), it's already modifier form
        if closing_loc.is_none() {
            return Vec::new();
        }

        let body = match statements {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_stmts: Vec<_> = body.body().iter().collect();
        if body_stmts.len() != 1 {
            return Vec::new();
        }

        let body_node = &body_stmts[0];

        // Body must be on a single line
        let (body_start_line, _) = source.offset_to_line_col(body_node.location().start_offset());
        let body_end_off = body_node.location().end_offset().saturating_sub(1).max(body_node.location().start_offset());
        let (body_end_line, _) = source.offset_to_line_col(body_end_off);
        if body_start_line != body_end_line {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Favor modifier `{}` usage when having a single-line body.", keyword),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhileUntilModifier, "cops/style/while_until_modifier");
}
