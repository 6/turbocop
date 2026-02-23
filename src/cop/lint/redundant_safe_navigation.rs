use crate::cop::node_type::{
    ARRAY_NODE, BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, FALSE_NODE,
    FLOAT_NODE, HASH_NODE, INTEGER_NODE, KEYWORD_HASH_NODE, REGULAR_EXPRESSION_NODE, SELF_NODE,
    STRING_NODE, SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantSafeNavigation;

/// Methods guaranteed to exist on every instance (their receivers can't be nil)
const GUARANTEED_INSTANCE_METHODS: &[&[u8]] = &[b"to_s", b"to_i", b"to_f", b"to_a", b"to_h"];

/// Methods that are allowed in conditions (default AllowedMethods)
const DEFAULT_ALLOWED_METHODS: &[&[u8]] = &[
    b"instance_of?",
    b"kind_of?",
    b"is_a?",
    b"eql?",
    b"respond_to?",
    b"equal?",
];

impl Cop for RedundantSafeNavigation {
    fn name(&self) -> &'static str {
        "Lint/RedundantSafeNavigation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            BLOCK_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            FALSE_NODE,
            FLOAT_NODE,
            HASH_NODE,
            INTEGER_NODE,
            KEYWORD_HASH_NODE,
            REGULAR_EXPRESSION_NODE,
            SELF_NODE,
            STRING_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
        ]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must use safe navigation (&.)
        let op_loc = match call.call_operator_loc() {
            Some(loc) if loc.as_slice() == b"&." => loc,
            _ => return,
        };

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let _method_name = call.name().as_slice();

        // Case 1: Receiver is a constant in camel case (not all uppercase/snake case)
        if is_camel_case_const(&receiver) {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant safe navigation detected, use `.` instead.".to_string(),
            ));
        }

        // Case 2: Receiver is a literal (not nil)
        if is_non_nil_literal(&receiver) {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant safe navigation detected, use `.` instead.".to_string(),
            ));
        }

        // Case 3: Receiver is `self`
        if receiver.as_self_node().is_some() {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant safe navigation detected, use `.` instead.".to_string(),
            ));
        }

        // Case 4: Receiver is a guaranteed instance method call (to_s, to_i, etc.)
        // foo.to_s&.strip is redundant because to_s always returns a string
        if is_guaranteed_instance_receiver(&receiver) {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant safe navigation detected, use `.` instead.".to_string(),
            ));
        }

        // Case 5: AllowedMethods used in conditions
        let allowed_methods = config.get_string_array("AllowedMethods");
        let is_allowed = if let Some(ref allowed) = allowed_methods {
            allowed.iter().any(|m| m.as_bytes() == _method_name)
        } else {
            DEFAULT_ALLOWED_METHODS.contains(&_method_name)
        };

        // Note: We'd need parent context to check if the call is in a condition.
        // For now, we only handle the simpler cases above.
        let _ = is_allowed;
    }
}

fn is_camel_case_const(node: &ruby_prism::Node<'_>) -> bool {
    let const_name = if let Some(c) = node.as_constant_read_node() {
        c.name().as_slice().to_vec()
    } else if let Some(cp) = node.as_constant_path_node() {
        // Use the last part of the path
        if let Some(name) = cp.name() {
            name.as_slice().to_vec()
        } else {
            return false;
        }
    } else {
        return false;
    };

    // All-uppercase or all-uppercase+underscore = snake case constant, not camel case
    // Check if it's NOT all uppercase/underscore/digits
    !const_name
        .iter()
        .all(|&b| b.is_ascii_uppercase() || b == b'_' || b.is_ascii_digit())
}

fn is_non_nil_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_regular_expression_node().is_some()
}

fn is_guaranteed_instance_receiver(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        // Don't check if the call itself uses safe navigation
        if let Some(op) = call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return false;
            }
        }
        let method = call.name().as_slice();
        GUARANTEED_INSTANCE_METHODS.contains(&method)
    } else if let Some(block) = node.as_block_node() {
        // Block wrapping: foo.to_h { ... }&.keys
        let src = block.location().as_slice();
        // Check if this block's source contains a guaranteed method with regular dot
        for method in GUARANTEED_INSTANCE_METHODS {
            let dot_method = [b"." as &[u8], *method].concat();
            if src
                .windows(dot_method.len())
                .any(|w| w == dot_method.as_slice())
            {
                // Make sure it doesn't use &.
                let safe_method = [b"&." as &[u8], *method].concat();
                if !src
                    .windows(safe_method.len())
                    .any(|w| w == safe_method.as_slice())
                {
                    return true;
                }
            }
        }
        false
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantSafeNavigation,
        "cops/lint/redundant_safe_navigation"
    );
}
