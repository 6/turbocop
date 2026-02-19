use crate::cop::util::is_blank_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{IF_NODE, UNTIL_NODE, WHILE_NODE};

pub struct EmptyLineAfterMultilineCondition;

impl Cop for EmptyLineAfterMultilineCondition {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineAfterMultilineCondition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNTIL_NODE, WHILE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check if/unless nodes
        if let Some(if_node) = node.as_if_node() {
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(),
            };
            let kw_slice = kw_loc.as_slice();
            if kw_slice != b"if" && kw_slice != b"unless" {
                return Vec::new();
            }
            let predicate = if_node.predicate();
            return self.check_multiline_condition(source, &kw_loc, &predicate);
        }

        // Check while nodes
        if let Some(while_node) = node.as_while_node() {
            let kw_loc = while_node.keyword_loc();
            if kw_loc.as_slice() != b"while" {
                return Vec::new();
            }
            let predicate = while_node.predicate();
            return self.check_multiline_condition(source, &kw_loc, &predicate);
        }

        // Check until nodes
        if let Some(until_node) = node.as_until_node() {
            let kw_loc = until_node.keyword_loc();
            if kw_loc.as_slice() != b"until" {
                return Vec::new();
            }
            let predicate = until_node.predicate();
            return self.check_multiline_condition(source, &kw_loc, &predicate);
        }

        Vec::new()
    }
}

impl EmptyLineAfterMultilineCondition {
    fn check_multiline_condition(
        &self,
        source: &SourceFile,
        kw_loc: &ruby_prism::Location<'_>,
        predicate: &ruby_prism::Node<'_>,
    ) -> Vec<Diagnostic> {
        let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
        let pred_end = predicate.location().end_offset().saturating_sub(1);
        let (pred_end_line, _) = source.offset_to_line_col(pred_end);

        // Only check multiline conditions
        if kw_line == pred_end_line {
            return Vec::new();
        }

        let lines: Vec<&[u8]> = source.lines().collect();
        // The line after the condition ends
        let next_line_num = pred_end_line + 1;
        if next_line_num > lines.len() {
            return Vec::new();
        }

        let next_line = lines[next_line_num - 1];
        if !is_blank_line(next_line) {
            let (line, col) = source.offset_to_line_col(kw_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                col,
                "Use an empty line after a multiline condition.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLineAfterMultilineCondition,
        "cops/layout/empty_line_after_multiline_condition"
    );
}
