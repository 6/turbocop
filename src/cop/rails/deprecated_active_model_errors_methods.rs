use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct DeprecatedActiveModelErrorsMethods;

const MSG: &str = "Avoid manipulating ActiveModel errors as hash directly.";

/// Manipulative methods that indicate direct hash manipulation.
const MANIPULATIVE_METHODS: &[&[u8]] = &[b"<<", b"clear"];

/// Deprecated methods called directly on errors (e.g., errors.keys, errors.values).
const DEPRECATED_ERRORS_METHODS: &[&[u8]] = &[b"keys", b"values", b"to_h", b"to_xml"];

impl Cop for DeprecatedActiveModelErrorsMethods {
    fn name(&self) -> &'static str {
        "Rails/DeprecatedActiveModelErrorsMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Pattern 1: errors[:field] << 'msg'  /  errors[:field].clear  /  errors[:field] = []
        // Pattern 2: errors.messages[:field] << 'msg'  /  etc.
        // Pattern 3: errors.details[:field] << 'msg'  /  etc.
        // Pattern 4: errors.keys  /  errors.values  /  errors.to_h  /  errors.to_xml

        let call = match node.as_call_node() {
            Some(c) => c,
            None => {
                // Check for assignment: errors[:name] = []
                // This would be a CallNode with name `[]=` in Prism
                return Vec::new();
            }
        };

        let method_name = call.name().as_slice();

        // Pattern 4: errors.keys / errors.values / errors.to_h / errors.to_xml
        if DEPRECATED_ERRORS_METHODS.contains(&method_name) {
            if let Some(recv) = call.receiver() {
                if is_errors_receiver(&recv) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(source, line, column, MSG.to_string())];
                }
            }
        }

        // Pattern: errors[:name] << 'msg' / errors[:name].clear / errors[:name] = []
        // Also: errors.messages[:name] << / errors.details[:name] <<
        if MANIPULATIVE_METHODS.contains(&method_name) || method_name == b"[]=" {
            if let Some(recv) = call.receiver() {
                if is_errors_bracket_access(&recv) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(source, line, column, MSG.to_string())];
                }
            }
        }

        // Pattern: errors[:name] = [] (Prism represents as `[]=` call)
        // Already handled above with `[]=` check

        Vec::new()
    }
}

/// Check if a node is `errors`, `errors.messages`, or `errors.details`.
fn is_errors_receiver(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if name == b"errors" {
            return true;
        }
        // errors.messages or errors.details
        if (name == b"messages" || name == b"details") && call.arguments().is_none() {
            if let Some(recv) = call.receiver() {
                return is_errors_call(&recv);
            }
        }
    }
    false
}

/// Check if a node is `x.errors` or bare `errors`.
fn is_errors_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        return call.name().as_slice() == b"errors";
    }
    false
}

/// Check if a node is `errors[:field]`, `errors.messages[:field]`, or `errors.details[:field]`.
fn is_errors_bracket_access(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"[]" {
            if let Some(recv) = call.receiver() {
                return is_errors_receiver(&recv);
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DeprecatedActiveModelErrorsMethods,
        "cops/rails/deprecated_active_model_errors_methods"
    );
}
