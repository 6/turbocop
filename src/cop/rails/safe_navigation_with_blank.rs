use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, IF_NODE, UNLESS_NODE};

pub struct SafeNavigationWithBlank;

fn check_safe_blank_predicate(
    source: &SourceFile,
    predicate: &ruby_prism::Node<'_>,
    cop: &SafeNavigationWithBlank,
) -> Vec<crate::diagnostic::Diagnostic> {
    let call = match predicate.as_call_node() {
        Some(c) => c,
        None => return Vec::new(),
    };

    if call.name().as_slice() != b"blank?" {
        return Vec::new();
    }

    // Check for safe navigation operator (&.)
    let call_op = match call.call_operator_loc() {
        Some(op) => op,
        None => return Vec::new(),
    };

    let op_bytes = &source.as_bytes()[call_op.start_offset()..call_op.end_offset()];
    if op_bytes != b"&." {
        return Vec::new();
    }

    let loc = call.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    vec![cop.diagnostic(
        source,
        line,
        column,
        "Avoid calling `blank?` with the safe navigation operator in conditionals.".to_string(),
    )]
}

impl Cop for SafeNavigationWithBlank {
    fn name(&self) -> &'static str {
        "Rails/SafeNavigationWithBlank"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
    ) {
        // Check if nodes
        if let Some(if_node) = node.as_if_node() {
            let predicate = if_node.predicate();
            diagnostics.extend(check_safe_blank_predicate(source, &predicate, self));
            return;
        }

        // Check unless nodes
        if let Some(unless_node) = node.as_unless_node() {
            let predicate = unless_node.predicate();
            diagnostics.extend(check_safe_blank_predicate(source, &predicate, self));
            return;
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SafeNavigationWithBlank, "cops/rails/safe_navigation_with_blank");
}
