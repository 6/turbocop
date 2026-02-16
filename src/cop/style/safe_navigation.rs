use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SafeNavigation;

impl SafeNavigation {
    /// Check if two nodes represent the same identifier, including:
    /// - local variable reads
    /// - instance/class/global variable reads
    /// - bare method calls (no receiver, no args) which look like variable reads
    fn same_identifier(a: &ruby_prism::Node<'_>, b: &ruby_prism::Node<'_>) -> bool {
        if let (Some(la), Some(lb)) = (a.as_local_variable_read_node(), b.as_local_variable_read_node()) {
            return la.name().as_slice() == lb.name().as_slice();
        }
        if let (Some(ia), Some(ib)) = (a.as_instance_variable_read_node(), b.as_instance_variable_read_node()) {
            return ia.name().as_slice() == ib.name().as_slice();
        }
        if let (Some(ca), Some(cb)) = (a.as_class_variable_read_node(), b.as_class_variable_read_node()) {
            return ca.name().as_slice() == cb.name().as_slice();
        }
        if let (Some(ga), Some(gb)) = (a.as_global_variable_read_node(), b.as_global_variable_read_node()) {
            return ga.name().as_slice() == gb.name().as_slice();
        }
        // Both are bare method calls (no receiver, no args) with the same name
        if let (Some(ca), Some(cb)) = (a.as_call_node(), b.as_call_node()) {
            if ca.receiver().is_none() && cb.receiver().is_none()
                && ca.arguments().is_none() && cb.arguments().is_none()
                && ca.block().is_none() && cb.block().is_none()
            {
                return ca.name().as_slice() == cb.name().as_slice();
            }
        }
        // One is a local variable read and the other is a bare method call with same name
        if let Some(lv) = a.as_local_variable_read_node() {
            if let Some(call) = b.as_call_node() {
                if call.receiver().is_none() && call.arguments().is_none() && call.block().is_none() {
                    return lv.name().as_slice() == call.name().as_slice();
                }
            }
        }
        if let Some(lv) = b.as_local_variable_read_node() {
            if let Some(call) = a.as_call_node() {
                if call.receiver().is_none() && call.arguments().is_none() && call.block().is_none() {
                    return lv.name().as_slice() == call.name().as_slice();
                }
            }
        }
        false
    }

    fn is_simple_identifier(node: &ruby_prism::Node<'_>) -> bool {
        if node.as_local_variable_read_node().is_some()
            || node.as_instance_variable_read_node().is_some()
            || node.as_class_variable_read_node().is_some()
            || node.as_global_variable_read_node().is_some()
        {
            return true;
        }
        // Bare method call (no receiver, no args) acts like a variable read
        if let Some(call) = node.as_call_node() {
            if call.receiver().is_none()
                && call.arguments().is_none()
                && call.block().is_none()
                && call.call_operator_loc().is_none()
            {
                return true;
            }
        }
        false
    }

    /// Check if the innermost receiver of a call chain matches a variable.
    /// Returns the chain depth if matched.
    fn matches_receiver_chain(node: &ruby_prism::Node<'_>, lhs: &ruby_prism::Node<'_>) -> Option<usize> {
        if let Some(call) = node.as_call_node() {
            if let Some(recv) = call.receiver() {
                if Self::same_identifier(&recv, lhs) {
                    return Some(1);
                }
                // Recurse into the receiver chain
                if let Some(depth) = Self::matches_receiver_chain(&recv, lhs) {
                    return Some(depth + 1);
                }
            }
        }
        None
    }
}

impl Cop for SafeNavigation {
    fn name(&self) -> &'static str {
        "Style/SafeNavigation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let max_chain_length = config.get_usize("MaxChainLength", 2);
        let _convert_nil = config.get_bool("ConvertCodeThatCanStartToReturnNil", false);
        let _allowed_methods = config.get_string_array("AllowedMethods");

        // Pattern: foo && foo.bar
        let and_node = match node.as_and_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let lhs = and_node.left();
        let rhs = and_node.right();

        // LHS must be a simple variable or bare method
        if !Self::is_simple_identifier(&lhs) {
            return Vec::new();
        }

        // RHS must be a method call chain
        let rhs_call = match rhs.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // The outermost call must use a dot operator
        if rhs_call.call_operator_loc().is_none() {
            return Vec::new();
        }

        // Check if the innermost receiver matches the LHS variable
        let chain_len = match Self::matches_receiver_chain(&rhs, &lhs) {
            Some(d) => d,
            None => return Vec::new(),
        };

        if chain_len > max_chain_length {
            return Vec::new();
        }

        // Check the outermost method name for unsafe patterns
        let name = rhs_call.name();
        let name_bytes = name.as_slice();

        // Skip nil methods like .nil? (safe nav would change semantics)
        if name_bytes == b"nil?" {
            return Vec::new();
        }
        // Skip .empty? in conditionals (nil&.empty? returns nil, not false)
        if name_bytes == b"empty?" {
            return Vec::new();
        }
        // Skip assignment methods
        if name_bytes.ends_with(b"=") && !name_bytes.ends_with(b"==") {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use safe navigation (`&.`) instead of checking if an object exists before calling the method.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SafeNavigation, "cops/style/safe_navigation");
}
