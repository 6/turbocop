use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTEGER_NODE, LOCAL_VARIABLE_READ_NODE};

/// Checks for numeric operations that have a constant result.
/// For example: `x * 0` always returns 0, `x / x` always returns 1.
pub struct NumericOperationWithConstantResult;

impl Cop for NumericOperationWithConstantResult {
    fn name(&self) -> &'static str {
        "Lint/NumericOperationWithConstantResult"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE, LOCAL_VARIABLE_READ_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Only check *, /, **
        if method_name != b"*" && method_name != b"/" && method_name != b"**" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args: Vec<_> = arguments.arguments().iter().collect();
        if args.len() != 1 {
            return;
        }

        let rhs = &args[0];

        let has_constant_result = if is_zero(rhs, source) {
            // x * 0 => 0, x ** 0 => 1
            method_name == b"*" || method_name == b"**"
        } else if method_name == b"/" && same_source(&receiver, rhs, source) {
            // x / x => 1
            true
        } else {
            false
        };

        if !has_constant_result {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Numeric operation with a constant result detected.".to_string(),
        ));
    }
}

fn is_zero(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    if let Some(int_node) = node.as_integer_node() {
        let src = &source.as_bytes()[int_node.location().start_offset()..int_node.location().end_offset()];
        return src == b"0";
    }
    false
}

fn same_source(a: &ruby_prism::Node<'_>, b: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    let a_src = &source.as_bytes()[a.location().start_offset()..a.location().end_offset()];
    let b_src = &source.as_bytes()[b.location().start_offset()..b.location().end_offset()];
    // Only compare simple variable reads (single identifier, not complex expressions)
    if a_src.len() > 50 || a_src.is_empty() {
        return false;
    }
    // Must be a simple local variable read or method call without args
    let a_is_simple = a.as_local_variable_read_node().is_some()
        || (a.as_call_node().is_some()
            && a.as_call_node().unwrap().receiver().is_none()
            && a.as_call_node().unwrap().arguments().is_none());
    if !a_is_simple {
        return false;
    }
    a_src == b_src
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NumericOperationWithConstantResult,
        "cops/lint/numeric_operation_with_constant_result"
    );
}
