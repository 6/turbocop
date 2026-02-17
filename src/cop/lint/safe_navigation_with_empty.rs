use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SafeNavigationWithEmpty;

impl Cop for SafeNavigationWithEmpty {
    fn name(&self) -> &'static str {
        "Lint/SafeNavigationWithEmpty"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for `foo&.empty?` used inside a conditional context
        // The call must use safe navigation (&.) and call `empty?`

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be calling `empty?`
        if call.name().as_slice() != b"empty?" {
            return Vec::new();
        }

        // Must use safe navigation operator (&.)
        let call_op = match call.call_operator_loc() {
            Some(op) => op,
            None => return Vec::new(),
        };

        if call_op.as_slice() != b"&." {
            return Vec::new();
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return Vec::new();
        }

        // We register an offense regardless of conditional context,
        // because `foo&.empty?` is always problematic:
        // if foo is nil, it returns nil (falsy), which means "not empty"
        // which is likely not the intended behavior.
        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid calling `empty?` with the safe navigation operator in conditionals.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SafeNavigationWithEmpty, "cops/lint/safe_navigation_with_empty");
}
