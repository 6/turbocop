use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, IF_NODE, UNLESS_NODE};

pub struct SafeNavigationWithEmpty;

impl Cop for SafeNavigationWithEmpty {
    fn name(&self) -> &'static str {
        "Lint/SafeNavigationWithEmpty"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, IF_NODE, UNLESS_NODE]
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
        // Look for `foo&.empty?` used inside a conditional (if/unless).
        // RuboCop's pattern: (if (csend (send ...) :empty?) ...)
        // We check IfNode whose condition is a &.empty? call.

        let if_node = if let Some(n) = node.as_if_node() {
            Some(n.predicate())
        } else if let Some(n) = node.as_unless_node() {
            Some(n.predicate())
        } else {
            None
        };

        let predicate = match if_node {
            Some(p) => p,
            None => return,
        };

        // Check if the condition is a &.empty? call
        let call = match predicate.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be calling `empty?`
        if call.name().as_slice() != b"empty?" {
            return;
        }

        // Must use safe navigation operator (&.)
        let call_op = match call.call_operator_loc() {
            Some(op) => op,
            None => return,
        };

        if call_op.as_slice() != b"&." {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid calling `empty?` with the safe navigation operator in conditionals.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SafeNavigationWithEmpty, "cops/lint/safe_navigation_with_empty");
}
