use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, ELSE_NODE, IF_NODE, TRUE_NODE};

pub struct RedundantCondition;

impl RedundantCondition {
    /// Check if two nodes represent the same source code
    fn nodes_equal(source: &SourceFile, a: &ruby_prism::Node<'_>, b: &ruby_prism::Node<'_>) -> bool {
        let a_bytes = &source.as_bytes()[a.location().start_offset()..a.location().start_offset() + a.location().as_slice().len()];
        let b_bytes = &source.as_bytes()[b.location().start_offset()..b.location().start_offset() + b.location().as_slice().len()];
        a_bytes == b_bytes
    }

    fn make_diagnostic(&self, source: &SourceFile, if_node: &ruby_prism::IfNode<'_>, msg: &str) -> Diagnostic {
        let loc = if let Some(kw) = if_node.if_keyword_loc() {
            kw.start_offset()
        } else {
            if_node.location().start_offset()
        };
        let (line, column) = source.offset_to_line_col(loc);
        self.diagnostic(source, line, column, msg.to_string())
    }
}

impl Cop for RedundantCondition {
    fn name(&self) -> &'static str {
        "Style/RedundantCondition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, ELSE_NODE, IF_NODE, TRUE_NODE]
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

        // Must have an else clause, not an elsif
        let subsequent = match if_node.subsequent() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // If the subsequent is another IfNode, it's an elsif — skip
        if subsequent.as_if_node().is_some() {
            return Vec::new();
        }

        // Vendor RuboCop only flags when else branch is single-line
        if let Some(else_node) = subsequent.as_else_node() {
            if let Some(else_stmts) = else_node.statements() {
                let else_loc = else_stmts.location();
                let (start_line, _) = source.offset_to_line_col(else_loc.start_offset());
                let end_offset = else_loc.start_offset() + else_loc.as_slice().len();
                let (end_line, _) = source.offset_to_line_col(if end_offset > 0 { end_offset - 1 } else { 0 });
                if start_line != end_line {
                    return Vec::new();
                }
            }
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
            return vec![Self::make_diagnostic(self, source, &if_node, "Use double pipes `||` instead.")];
        }

        // Check: `x.predicate? ? true : x` or `if x.nil?; true; else; x; end`
        // Condition is a predicate call (ends in ?), true branch is literal `true`
        if true_value.as_true_node().is_some() {
            if let Some(call) = condition.as_call_node() {
                let method_name = call.name().as_slice();
                if method_name.ends_with(b"?") {
                    let allowed = config.get_string_array("AllowedMethods").unwrap_or_default();
                    let method_str = std::str::from_utf8(method_name).unwrap_or("");
                    let is_allowed = allowed.iter().any(|m| m == method_str);
                    if !is_allowed {
                        return vec![Self::make_diagnostic(self, source, &if_node, "Use double pipes `||` instead.")];
                    }
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantCondition, "cops/style/redundant_condition");
}
