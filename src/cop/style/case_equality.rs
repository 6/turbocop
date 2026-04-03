use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_REGULAR_EXPRESSION_NODE,
    REGULAR_EXPRESSION_NODE, SELF_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/CaseEquality: Avoid the case equality operator `===`.
///
/// Investigation: RuboCop unconditionally skips `===` when the receiver is a
/// non-module-name constant (ALL_CAPS like `NUMERIC_PATTERN`), regardless of
/// `AllowOnConstant`. Only PascalCase constants (e.g., `String`, `Integer`)
/// are subject to `AllowOnConstant`. This was causing 42 FPs across 27 repos
/// on patterns like `NUMERIC_PATTERN === timezone`.
///
/// Second fix: `receiver_constant_name()` was returning hardcoded "QualifiedPath"
/// for all `ConstantPathNode` receivers, which always passed `is_module_name()`.
/// Fixed to extract the actual last-segment name via `cp.name()`. This resolves
/// 21 FPs on patterns like `Constants::ATOM_UNSAFE === str` and `URI::HTTPS === @uri`
/// where the last segment is ALL_CAPS (not a module name).
///
/// Third fix: RuboCop's node matcher `(send $_ :=== $_)` expects exactly one argument
/// child after `:===` (in the parser gem, `&bl` block_pass counts as a separate child).
/// Calls like `native.===(*args, &bl)` have 2 children and don't match. In Prism,
/// block_pass is in `call_node.block()`, so we reconstruct the total argument count
/// and skip when it's not exactly 1. Fixes 1 FP in `enspirit/finitio-rb`.
pub struct CaseEquality;

impl CaseEquality {
    /// Extract the constant name from a receiver node (ConstantReadNode or ConstantPathNode).
    fn receiver_constant_name(node: &ruby_prism::Node<'_>) -> Option<String> {
        if let Some(c) = node.as_constant_read_node() {
            return Some(String::from_utf8_lossy(c.name().as_slice()).into_owned());
        }
        if let Some(cp) = node.as_constant_path_node() {
            // For qualified constants like Foo::Bar, extract the last segment name.
            // RuboCop checks the last segment: URI::HTTPS is ALL_CAPS (not a module name),
            // while URI::Generic is PascalCase (a module name).
            if let Some(name) = cp.name() {
                return Some(String::from_utf8_lossy(name.as_slice()).into_owned());
            }
            return None;
        }
        None
    }

    /// A "module name" constant has at least one lowercase ASCII letter (PascalCase).
    /// ALL_CAPS_CONSTANTS like NUMERIC_PATTERN are not module names.
    fn is_module_name(name: &str) -> bool {
        name.bytes().any(|b| b.is_ascii_lowercase())
    }
}

impl Cop for CaseEquality {
    fn name(&self) -> &'static str {
        "Style/CaseEquality"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
            REGULAR_EXPRESSION_NODE,
            SELF_NODE,
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
        let allow_on_constant = config.get_bool("AllowOnConstant", false);
        let allow_on_self_class = config.get_bool("AllowOnSelfClass", false);

        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call_node.name().as_slice() != b"===" {
            return;
        }

        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return,
        };

        // RuboCop's matcher `(send $_ :=== $_)` expects exactly one argument child
        // (in the parser gem, block_pass counts as a separate child of `send`).
        // In Prism, block_pass is in `call_node.block()`, so we reconstruct the total.
        // This skips patterns like `native.===(*args, &bl)` which have 2 "children".
        let arg_count = call_node.arguments().map_or(0, |a| a.arguments().len());
        let has_block_arg = call_node
            .block()
            .is_some_and(|b| b.as_block_argument_node().is_some());
        let total_args = arg_count + usize::from(has_block_arg);
        if total_args != 1 {
            return;
        }

        // Skip regexp receivers (Performance/RegexpMatch handles those)
        if receiver.as_regular_expression_node().is_some()
            || receiver.as_interpolated_regular_expression_node().is_some()
        {
            return;
        }

        // RuboCop unconditionally skips constants that are not "module names"
        // (i.e., ALL_CAPS like NUMERIC_PATTERN). Only PascalCase constants
        // (like String, Integer) are subject to the AllowOnConstant setting.
        if let Some(const_name) = Self::receiver_constant_name(&receiver) {
            if !Self::is_module_name(&const_name) {
                return;
            }
            if allow_on_constant {
                return;
            }
        }

        // AllowOnSelfClass: self.class === something
        if allow_on_self_class {
            if let Some(recv_call) = receiver.as_call_node() {
                if recv_call.name().as_slice() == b"class" {
                    if let Some(inner_recv) = recv_call.receiver() {
                        if inner_recv.as_self_node().is_some() {
                            return;
                        }
                    }
                }
            }
        }

        let msg_loc = call_node
            .message_loc()
            .unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid the use of the case equality operator `===`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CaseEquality, "cops/style/case_equality");
}
