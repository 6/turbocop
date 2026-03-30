use crate::cop::node_type::{ARRAY_NODE, SPLAT_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Flags `[*var]` splat coercion and explicit `var = [var] unless var.is_a?(Array)`.
///
/// Fixed false negatives by matching Prism `UnlessNode` explicit array checks. The
/// previous implementation only listened to array/splat nodes, so same-variable
/// coercions like `groups = [groups] unless groups.is_a?(Array)` were never visited.
pub struct ArrayCoercion;

impl Cop for ArrayCoercion {
    fn name(&self) -> &'static str {
        "Style/ArrayCoercion"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, SPLAT_NODE, UNLESS_NODE]
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
        // Pattern 1: [*var] - splat into array with single element
        if let Some(array_node) = node.as_array_node() {
            // Skip implicit arrays (e.g., RHS of multi-write `a, b = *x`)
            if array_node.opening_loc().is_none() {
                return;
            }
            let elements: Vec<_> = array_node.elements().iter().collect();
            if elements.len() == 1 && elements[0].as_splat_node().is_some() {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `Array(variable)` instead of `[*variable]`.".to_string(),
                ));
            }
        }

        if let Some(unless_node) = node.as_unless_node() {
            if let Some(diagnostic) = self.check_explicit_array_check(source, &unless_node) {
                diagnostics.push(diagnostic);
            }
        }
    }
}

impl ArrayCoercion {
    fn check_explicit_array_check(
        &self,
        source: &SourceFile,
        unless_node: &ruby_prism::UnlessNode<'_>,
    ) -> Option<Diagnostic> {
        if unless_node.else_clause().is_some() {
            return None;
        }

        let predicate = unless_node.predicate().as_call_node()?;
        if predicate.name().as_slice() != b"is_a?" {
            return None;
        }

        let receiver = predicate.receiver()?.as_local_variable_read_node()?;
        let arguments = predicate.arguments()?;
        let args: Vec<_> = arguments.arguments().iter().collect();
        if args.len() != 1 {
            return None;
        }
        // RuboCop's pattern is `(const nil? :Array)` which only matches bare `Array`,
        // not `::Array` (constant_path_node). Intentionally not handled.
        let constant = args[0].as_constant_read_node()?;
        if constant.name().as_slice() != b"Array" {
            return None;
        }

        let statements = unless_node.statements()?;
        let body: Vec<_> = statements.body().iter().collect();
        if body.len() != 1 {
            return None;
        }

        let assignment = body[0].as_local_variable_write_node()?;
        let assigned_name = assignment.name().as_slice();
        if assigned_name != receiver.name().as_slice() {
            return None;
        }

        let array = assignment.value().as_array_node()?;
        array.opening_loc()?;

        let elements: Vec<_> = array.elements().iter().collect();
        if elements.len() != 1 {
            return None;
        }

        let element = elements[0].as_local_variable_read_node()?;
        if element.name().as_slice() != assigned_name {
            return None;
        }

        let loc = unless_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        Some(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `Array({})` instead of explicit `Array` check.",
                String::from_utf8_lossy(assigned_name)
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArrayCoercion, "cops/style/array_coercion");
}
