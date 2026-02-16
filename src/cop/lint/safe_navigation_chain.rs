use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SafeNavigationChain;

/// Methods that are safe to call on nil (NilMethods) or commonly allowed after &.
const DEFAULT_ALLOWED: &[&[u8]] = &[
    b"present?",
    b"blank?",
    b"presence",
    b"try",
    b"try!",
    b"nil?",
    b"to_d",
    b"in?",
];

impl Cop for SafeNavigationChain {
    fn name(&self) -> &'static str {
        "Lint/SafeNavigationChain"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // This call must NOT use safe navigation itself
        if let Some(op) = call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return Vec::new(); // This call itself is safe navigation
            }
        }

        // Check if the receiver used safe navigation
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if !receiver_uses_safe_nav(&receiver) {
            return Vec::new();
        }

        let method_name = call.name().as_slice();

        // Check allowed methods
        let allowed_methods = config.get_string_array("AllowedMethods");
        let is_allowed = if let Some(ref allowed) = allowed_methods {
            allowed.iter().any(|m| m.as_bytes() == method_name)
        } else {
            DEFAULT_ALLOWED.iter().any(|&m| m == method_name)
        };

        if is_allowed {
            return Vec::new();
        }

        // Skip unary +@ and -@ operators
        if method_name == b"+@" || method_name == b"-@" {
            return Vec::new();
        }

        // Skip assignment methods (foo= etc.) but not comparison operators
        if method_name.ends_with(b"=")
            && method_name != b"=="
            && method_name != b"==="
            && method_name != b"!="
            && method_name != b"<="
            && method_name != b">="
        {
            return Vec::new();
        }

        // Skip ==, ===, !=, |, & (these are valid after safe navigation)
        if method_name == b"=="
            || method_name == b"==="
            || method_name == b"!="
            || method_name == b"|"
            || method_name == b"&"
        {
            return Vec::new();
        }

        // Report at the dot or after the receiver
        let (line, column) = if let Some(dot_loc) = call.call_operator_loc() {
            source.offset_to_line_col(dot_loc.start_offset())
        } else {
            // Operator call with no dot — report at the receiver end
            let recv_end = receiver.location().end_offset();
            source.offset_to_line_col(recv_end)
        };

        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not chain ordinary method call after safe navigation operator.".to_string(),
        )]
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
