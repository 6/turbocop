use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INDEX_OR_WRITE_NODE, NIL_NODE, STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation: 2 FN from `ENV['HOME'] ||= value` pattern (net-ssh).
/// Prism parses `ENV['HOME'] ||= value` as `IndexOrWriteNode`, not `CallNode`.
/// Fixed by adding `IndexOrWriteNode` handling alongside the existing `CallNode` path.
pub struct EnvHome;

impl Cop for EnvHome {
    fn name(&self) -> &'static str {
        "Style/EnvHome"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INDEX_OR_WRITE_NODE,
            NIL_NODE,
            STRING_NODE,
        ]
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
        // Handle ENV['HOME'] ||= value (IndexOrWriteNode)
        if let Some(write) = node.as_index_or_write_node() {
            if let Some(receiver) = write.receiver() {
                if is_env_receiver(&receiver) && has_home_first_arg(write.arguments()) {
                    let start = receiver.location().start_offset();
                    let (line, column) = source.offset_to_line_col(start);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `Dir.home` instead.".to_string(),
                    ));
                }
            }
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Must be [] or fetch
        if method_bytes != b"[]" && method_bytes != b"fetch" {
            return;
        }

        // Receiver must be ENV constant
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !is_env_receiver(&receiver) {
            return;
        }

        // First argument must be string "HOME"
        if !has_home_first_arg(call.arguments()) {
            return;
        }

        // For fetch, second arg must be nil or absent
        if method_bytes == b"fetch" {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() == 2 && arg_list[1].as_nil_node().is_none() {
                    return;
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `Dir.home` instead.".to_string(),
        ));
    }
}

fn is_env_receiver(receiver: &ruby_prism::Node<'_>) -> bool {
    receiver
        .as_constant_read_node()
        .is_some_and(|c| c.name().as_slice() == b"ENV")
        || receiver.as_constant_path_node().is_some_and(|cp| {
            cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"ENV")
        })
}

fn has_home_first_arg(arguments: Option<ruby_prism::ArgumentsNode<'_>>) -> bool {
    let args = match arguments {
        Some(a) => a,
        None => return false,
    };
    let mut iter = args.arguments().iter();
    match iter.next() {
        Some(first_arg) => first_arg
            .as_string_node()
            .is_some_and(|s| s.unescaped() == b"HOME"),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnvHome, "cops/style/env_home");
}
