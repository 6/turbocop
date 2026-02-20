use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{AND_NODE, IF_NODE, OR_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE};

pub struct AndOr;

impl Cop for AndOr {
    fn name(&self) -> &'static str {
        "Style/AndOr"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, IF_NODE, OR_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "conditionals");

        if enforced_style == "always" {
            // In "always" mode, flag every `and` and `or` keyword
            diagnostics.extend(check_and_or_node(self, source, node));
            return;
        }

        // "conditionals" mode: only flag `and`/`or` inside conditions of if/while/until
        let condition = if let Some(if_node) = node.as_if_node() {
            if_node.predicate()
        } else if let Some(unless_node) = node.as_unless_node() {
            unless_node.predicate()
        } else if let Some(while_node) = node.as_while_node() {
            while_node.predicate()
        } else if let Some(until_node) = node.as_until_node() {
            until_node.predicate()
        } else {
            return;
        };

        // Walk the condition tree for and/or nodes
        collect_and_or_in_condition(self, source, &condition, diagnostics);
    }
}

/// Check if a single node is an `and`/`or` keyword and report it.
fn check_and_or_node(cop: &AndOr, source: &SourceFile, node: &ruby_prism::Node<'_>) -> Vec<Diagnostic> {
    if let Some(and_node) = node.as_and_node() {
        let op_loc = and_node.operator_loc();
        if op_loc.as_slice() == b"and" {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            return vec![cop.diagnostic(
                source,
                line,
                column,
                "Use `&&` instead of `and`.".to_string(),
            )];
        }
    } else if let Some(or_node) = node.as_or_node() {
        let op_loc = or_node.operator_loc();
        if op_loc.as_slice() == b"or" {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            return vec![cop.diagnostic(
                source,
                line,
                column,
                "Use `||` instead of `or`.".to_string(),
            )];
        }
    }
    Vec::new()
}

/// Recursively walk a condition expression finding `and`/`or` keyword nodes.
fn collect_and_or_in_condition(
    cop: &AndOr,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(and_node) = node.as_and_node() {
        let op_loc = and_node.operator_loc();
        if op_loc.as_slice() == b"and" {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                "Use `&&` instead of `and`.".to_string(),
            ));
        }
        // Recurse into both sides
        collect_and_or_in_condition(cop, source, &and_node.left(), diagnostics);
        collect_and_or_in_condition(cop, source, &and_node.right(), diagnostics);
    } else if let Some(or_node) = node.as_or_node() {
        let op_loc = or_node.operator_loc();
        if op_loc.as_slice() == b"or" {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                "Use `||` instead of `or`.".to_string(),
            ));
        }
        // Recurse into both sides
        collect_and_or_in_condition(cop, source, &or_node.left(), diagnostics);
        collect_and_or_in_condition(cop, source, &or_node.right(), diagnostics);
    }
    // For other node types, don't recurse further — and/or at the top level of
    // a condition is what we're looking for.
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AndOr, "cops/style/and_or");
}
