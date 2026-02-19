use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, OR_NODE};

/// Checks for useless OR expressions where the left side always returns a truthy value.
pub struct UselessOr;

const TRUTHY_METHODS: &[&[u8]] = &[
    b"to_a", b"to_c", b"to_d", b"to_i", b"to_f", b"to_h", b"to_r", b"to_s", b"to_sym",
    b"intern", b"inspect", b"hash", b"object_id", b"__id__",
];

impl Cop for UselessOr {
    fn name(&self) -> &'static str {
        "Lint/UselessOr"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, OR_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let or_node = match node.as_or_node() {
            Some(n) => n,
            None => return,
        };

        let lhs = or_node.left();
        if is_truthy_method_call(&lhs) {
            let lhs_src = node_source(source, &lhs);
            let rhs_src = node_source(source, &or_node.right());
            let loc = or_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!(
                    "`{}` will never evaluate because `{}` always returns a truthy value.",
                    rhs_src, lhs_src
                ),
            ));
        }

    }
}

fn is_truthy_method_call(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    // Must have a receiver (not a bare method call)
    if call.receiver().is_none() {
        return false;
    }

    // Must have no arguments
    if call.arguments().is_some() {
        return false;
    }

    // Must not be safe navigation (&.) - safe navigation can return nil
    if let Some(op) = call.call_operator_loc() {
        if op.as_slice() == b"&." {
            return false;
        }
    }

    let method_name = call.name().as_slice();
    TRUTHY_METHODS.iter().any(|m| *m == method_name)
}

fn node_source<'a>(source: &'a SourceFile, node: &ruby_prism::Node<'_>) -> &'a str {
    let loc = node.location();
    std::str::from_utf8(&source.as_bytes()[loc.start_offset()..loc.end_offset()]).unwrap_or("...")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessOr, "cops/lint/useless_or");
}
