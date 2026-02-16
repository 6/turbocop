use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantCondition;

impl RedundantCondition {
    /// Check if two nodes represent the same source code
    fn nodes_equal(source: &SourceFile, a: &ruby_prism::Node<'_>, b: &ruby_prism::Node<'_>) -> bool {
        let a_bytes = &source.as_bytes()[a.location().start_offset()..a.location().start_offset() + a.location().as_slice().len()];
        let b_bytes = &source.as_bytes()[b.location().start_offset()..b.location().start_offset() + b.location().as_slice().len()];
        a_bytes == b_bytes
    }
}

impl Cop for RedundantCondition {
    fn name(&self) -> &'static str {
        "Style/RedundantCondition"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _allowed_methods = config.get_string_array("AllowedMethods");

        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Skip unless/elsif
        if let Some(kw_loc) = if_node.if_keyword_loc() {
            let kw_text = kw_loc.as_slice();
            if kw_text != b"if" {
                return Vec::new();
            }
        }

        // Must have an else clause (subsequent IfNode)
        if if_node.subsequent().is_none() {
            return Vec::new();
        }

        // Get the true branch (statements)
        let true_branch = match if_node.statements() {
            Some(stmts) => stmts,
            None => return Vec::new(),
        };

        let true_body: Vec<_> = true_branch.body().into_iter().collect();
        if true_body.len() != 1 {
            return Vec::new();
        }

        let condition = if_node.predicate();
        let true_value = &true_body[0];

        // Check: `x ? x : y` or `if x; x; else; y; end` where true branch equals condition
        if Self::nodes_equal(source, &condition, true_value) {
            let loc = if let Some(kw) = if_node.if_keyword_loc() {
                kw.start_offset()
            } else {
                if_node.location().start_offset()
            };
            let (line, column) = source.offset_to_line_col(loc);
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use double pipes `||` instead.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantCondition, "cops/style/redundant_condition");
}
