use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SafeNavigationChain;

/// Methods that are safe to call on nil (NilClass methods + Object/Kernel/BasicObject methods).
/// This mirrors RuboCop's NilMethods mixin which uses `nil.methods` at runtime.
/// Generated from `nil.methods.sort` in Ruby 3.3.
const NIL_METHODS: &[&[u8]] = &[
    // NilClass instance methods
    b"!",
    b"&",
    b"|",
    b"^",
    b"nil?",
    b"to_a",
    b"to_c",
    b"to_f",
    b"to_h",
    b"to_i",
    b"to_r",
    b"to_s",
    b"inspect",
    b"rationalize",
    // BasicObject methods
    b"==",
    b"===",
    b"!=",
    b"equal?",
    b"__id__",
    b"__send__",
    b"instance_eval",
    b"instance_exec",
    // Kernel / Object methods (available on every object including nil)
    b"<=>",
    b"=~",
    b"!~",
    b"eql?",
    b"hash",
    b"class",
    b"singleton_class",
    b"frozen?",
    b"is_a?",
    b"kind_of?",
    b"instance_of?",
    b"respond_to?",
    b"respond_to_missing?",
    b"send",
    b"public_send",
    b"object_id",
    b"dup",
    b"clone",
    b"freeze",
    b"tap",
    b"then",
    b"yield_self",
    b"itself",
    b"display",
    b"method",
    b"public_method",
    b"singleton_method",
    b"define_singleton_method",
    b"extend",
    b"to_enum",
    b"enum_for",
    b"instance_variable_get",
    b"instance_variable_set",
    b"instance_variable_defined?",
    b"remove_instance_variable",
    b"instance_variables",
    b"methods",
    b"private_methods",
    b"protected_methods",
    b"public_methods",
    b"singleton_methods",
    b"taint",
    b"untaint",
    b"tainted?",
    b"trust",
    b"untrust",
    b"untrusted?",
    b"pp",
    b"pretty_print",
    b"pretty_print_cycle",
    b"pretty_print_instance_variables",
    b"pretty_print_inspect",
    b"pretty_inspect",
    b"to_json",
    b"to_yaml",
    b"to_yaml_properties",
    b"psych_to_yaml",
    b"object_group",
    // stdlib extras
    b"to_d",
    // Default AllowedMethods from vendor config
    b"present?",
    b"blank?",
    b"presence",
    b"try",
    b"try!",
    b"in?",
];

impl Cop for SafeNavigationChain {
    fn name(&self) -> &'static str {
        "Lint/SafeNavigationChain"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE]
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

        // This call must NOT use safe navigation itself
        if let Some(op) = call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return; // This call itself is safe navigation
            }
        }

        // Check if the receiver used safe navigation
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !receiver_uses_safe_nav(&receiver) {
            return;
        }

        let method_name = call.name().as_slice();

        // Check allowed methods: nil methods are ALWAYS allowed, plus any configured AllowedMethods
        let is_nil_method = NIL_METHODS.contains(&method_name);
        if is_nil_method {
            return;
        }

        if let Some(ref allowed) = config.get_string_array("AllowedMethods") {
            if allowed.iter().any(|m| m.as_bytes() == method_name) {
                return;
            }
        }

        // Skip unary +@ and -@ operators
        if method_name == b"+@" || method_name == b"-@" {
            return;
        }

        // Skip assignment methods (foo= etc.) but not comparison operators
        if method_name.ends_with(b"=")
            && method_name != b"=="
            && method_name != b"==="
            && method_name != b"!="
            && method_name != b"<="
            && method_name != b">="
        {
            return;
        }

        // Skip ==, ===, !=, |, & (these are valid after safe navigation)
        if method_name == b"=="
            || method_name == b"==="
            || method_name == b"!="
            || method_name == b"|"
            || method_name == b"&"
        {
            return;
        }

        // Report at the dot or after the receiver
        let (line, column) = if let Some(dot_loc) = call.call_operator_loc() {
            source.offset_to_line_col(dot_loc.start_offset())
        } else {
            // Operator call with no dot — report at the receiver end
            let recv_end = receiver.location().end_offset();
            source.offset_to_line_col(recv_end)
        };

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not chain ordinary method call after safe navigation operator.".to_string(),
        ));
    }
}

fn receiver_uses_safe_nav(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(recv_call) = node.as_call_node() {
        recv_call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.")
    } else if let Some(block) = node.as_block_node() {
        // Block wrapping a csend: x&.select { ... }.bar
        let recv_src = block.location().as_slice();
        recv_src.windows(2).any(|w| w == b"&.")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SafeNavigationChain, "cops/lint/safe_navigation_chain");
}
