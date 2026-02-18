use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NegatedIfElseCondition;

impl NegatedIfElseCondition {
    fn is_negated(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            let name = call.name();
            if name.as_slice() == b"!" {
                return true;
            }
            // Check for `not` keyword
            if let Some(msg_loc) = call.message_loc() {
                if msg_loc.as_slice() == b"not" {
                    return true;
                }
            }
        }
        false
    }
}

impl Cop for NegatedIfElseCondition {
    fn name(&self) -> &'static str {
        "Style/NegatedIfElseCondition"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check if-else with negated condition
        if let Some(if_node) = node.as_if_node() {
            // Must have both branches (if + else, not just if)
            if if_node.statements().is_none() || if_node.subsequent().is_none() {
                return Vec::new();
            }

            // Determine if ternary (no if_keyword_loc in Prism) or regular if
            let is_ternary = if_node.if_keyword_loc().is_none();

            if !is_ternary {
                let kw = if_node.if_keyword_loc().unwrap();
                let kw_bytes = kw.as_slice();
                // Must be `if`, not `unless`
                if kw_bytes == b"unless" {
                    return Vec::new();
                }
            }

            // Check the subsequent is an else (not elsif)
            if let Some(sub) = if_node.subsequent() {
                if sub.as_else_node().is_none() {
                    return Vec::new();
                }
            }

            let predicate = if_node.predicate();
            if Self::is_negated(&predicate) {
                let loc = if_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let msg = if is_ternary {
                    "Invert the negated condition and swap the ternary branches."
                } else {
                    "Invert the negated condition and swap the if-else branches."
                };
                return vec![self.diagnostic(source, line, column, msg.to_string())];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NegatedIfElseCondition, "cops/style/negated_if_else_condition");
}
