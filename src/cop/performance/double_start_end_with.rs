use crate::cop::node_type::{AND_NODE, CALL_NODE, OR_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DoubleStartEndWith;

/// Check whether a Prism AST node represents a "pure" expression (no side effects).
/// Mirrors RuboCop's `Node#pure?` method from rubocop-ast.
fn is_pure(node: &ruby_prism::Node<'_>) -> bool {
    // Literals and variable reads are always pure
    if node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
        || node.as_constant_read_node().is_some()
        || node.as_constant_path_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_source_file_node().is_some()
        || node.as_source_line_node().is_some()
    {
        return true;
    }

    // Interpolated strings/symbols: pure only if all parts are pure
    // (In practice, interpolation with method calls like `"#{foo.bar}"` is impure)
    if let Some(interp) = node.as_interpolated_string_node() {
        return interp.parts().iter().all(|part| is_pure(&part));
    }
    if let Some(interp) = node.as_interpolated_symbol_node() {
        return interp.parts().iter().all(|part| is_pure(&part));
    }

    // String interpolation content nodes — embedded statements are impure
    // unless the contained statements are all pure
    if let Some(embedded) = node.as_embedded_statements_node() {
        if let Some(stmts) = embedded.statements() {
            return stmts.body().iter().all(|s| is_pure(&s));
        }
        return true;
    }

    // Array literals are pure if all elements are pure
    if let Some(arr) = node.as_array_node() {
        return arr.elements().iter().all(|e| is_pure(&e));
    }

    // Everything else (method calls, binary ops, etc.) is impure
    false
}

/// Extract the receiver of a CallNode, potentially unwrapping a `!` (not) prefix.
/// Returns `(call_node, is_negated)` where `call_node` is the inner start_with?/end_with? call.
fn unwrap_call<'a>(node: &ruby_prism::Node<'a>) -> Option<(ruby_prism::CallNode<'a>, bool)> {
    let call = node.as_call_node()?;
    let method = call.name().as_slice();

    // Check if this is `!expr` — in Prism that's a CallNode with method `!` and receiver
    if method == b"!" {
        let inner = call.receiver()?;
        let inner_call = inner.as_call_node()?;
        Some((inner_call, true))
    } else {
        Some((call, false))
    }
}

/// Check if all arguments of a call are pure.
fn all_args_pure(call: &ruby_prism::CallNode<'_>) -> bool {
    match call.arguments() {
        Some(args) => args.arguments().iter().all(|a| is_pure(&a)),
        None => true,
    }
}

impl Cop for DoubleStartEndWith {
    fn name(&self) -> &'static str {
        "Performance/DoubleStartEndWith"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, OR_NODE, AND_NODE]
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
        let include_as_aliases = config.get_bool("IncludeActiveSupportAliases", false);

        // Handle `||` pattern: x.start_with?(a) || x.start_with?(b)
        if let Some(or_node) = node.as_or_node() {
            let left_call = match or_node.left().as_call_node() {
                Some(c) => c,
                None => return,
            };
            let right_call = match or_node.right().as_call_node() {
                Some(c) => c,
                None => return,
            };

            if let Some(diag) =
                self.check_pair(source, &left_call, &right_call, include_as_aliases, node)
            {
                diagnostics.push(diag);
            }
            return;
        }

        // Handle `&&` pattern with negation: !x.start_with?(a) && !x.start_with?(b)
        if let Some(and_node) = node.as_and_node() {
            let (left_inner, left_neg) = match unwrap_call(&and_node.left()) {
                Some(v) => v,
                None => return,
            };
            let (right_inner, right_neg) = match unwrap_call(&and_node.right()) {
                Some(v) => v,
                None => return,
            };

            // Both sides must be negated
            if !left_neg || !right_neg {
                return;
            }

            if let Some(diag) =
                self.check_pair(source, &left_inner, &right_inner, include_as_aliases, node)
            {
                diagnostics.push(diag);
            }
        }
    }
}

impl DoubleStartEndWith {
    fn check_pair(
        &self,
        source: &SourceFile,
        left_call: &ruby_prism::CallNode<'_>,
        right_call: &ruby_prism::CallNode<'_>,
        include_as_aliases: bool,
        node: &ruby_prism::Node<'_>,
    ) -> Option<Diagnostic> {
        let left_name = left_call.name().as_slice();
        let right_name = right_call.name().as_slice();

        // Both sides must use the same method
        if left_name != right_name {
            return None;
        }

        let is_target = left_name == b"start_with?"
            || left_name == b"end_with?"
            || (include_as_aliases && (left_name == b"starts_with?" || left_name == b"ends_with?"));
        if !is_target {
            return None;
        }

        // Both sides must have the same receiver
        let left_receiver = left_call.receiver()?;
        let right_receiver = right_call.receiver()?;
        if left_receiver.location().as_slice() != right_receiver.location().as_slice() {
            return None;
        }

        // All arguments of the second call must be pure (no method calls, interpolation, etc.)
        if !all_args_pure(right_call) {
            return None;
        }

        let method_display = if left_name == b"start_with?" || left_name == b"starts_with?" {
            "start_with?"
        } else {
            "end_with?"
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        Some(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{method_display}` with multiple arguments instead of chaining `||`."),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DoubleStartEndWith, "cops/performance/double_start_end_with");
}
