use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashSet;

/// Checks for duplicate literal, constant, or variable elements in Set.
pub struct DuplicateSetElement;

impl Cop for DuplicateSetElement {
    fn name(&self) -> &'static str {
        "Lint/DuplicateSetElement"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        let elements = extract_set_elements(&call, source, method_name);
        let elements = match elements {
            Some(e) => e,
            None => return Vec::new(),
        };

        // Determine the class name for the message
        let class_name = if let Some(recv) = call.receiver() {
            if let Some(name) = constant_name(&recv) {
                std::str::from_utf8(name).unwrap_or("Set").to_string()
            } else {
                "Set".to_string()
            }
        } else {
            "Set".to_string()
        };

        let mut seen = HashSet::new();
        let mut diagnostics = Vec::new();

        for elem in &elements {
            // Only check literals, constants, and variables
            if !is_literal_const_or_var(elem) {
                continue;
            }

            let elem_src = &source.as_bytes()[elem.location().start_offset()..elem.location().end_offset()];
            let key = elem_src.to_vec();

            if !seen.insert(key) {
                let loc = elem.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Remove the duplicate element in {}.", class_name),
                ));
            }
        }

        diagnostics
    }
}

fn extract_set_elements<'pr>(
    call: &ruby_prism::CallNode<'pr>,
    _source: &SourceFile,
    method_name: &[u8],
) -> Option<Vec<ruby_prism::Node<'pr>>> {
    if let Some(recv) = call.receiver() {
        // Check for .to_set on an array literal first (before constant_name check)
        if method_name == b"to_set" {
            if let Some(array) = recv.as_array_node() {
                return Some(array.elements().iter().collect());
            }
        }

        let name = match constant_name(&recv) {
            Some(n) => n,
            None => return None,
        };
        if name != b"Set" && name != b"SortedSet" {
            return None;
        }

        if method_name == b"[]" {
            // Set[:foo, :bar, :foo]
            let args = call.arguments()?;
            return Some(args.arguments().iter().collect());
        } else if method_name == b"new" {
            // Set.new([:foo, :bar, :foo])
            let args = call.arguments()?;
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() == 1 {
                let array = arg_list[0].as_array_node()?;
                return Some(array.elements().iter().collect());
            }
        }
    }

    None
}

fn is_literal_const_or_var(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_constant_read_node().is_some()
        || node.as_constant_path_node().is_some()
        || node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateSetElement, "cops/lint/duplicate_set_element");
}
