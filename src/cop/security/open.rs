use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE, STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle reported FP=4, FN=3. FP fixed by handling `__FILE__` (SourceFileNode).
/// FN=3 were from lines with `# standard:disable Security/Open` comments. Fixed by
/// parsing only `rubocop:`/`nitrocop:` directives in `parse/directives.rs`, matching
/// RuboCop's source-directive behavior.
///
/// ## Corpus investigation (2026-03-20) — extended corpus
///
/// Extended corpus oracle reported FP=0, FN=1.
///
/// FN=1: Fixed by also flagging `open(&block)` calls where only a block argument
/// (no positional args) is passed. In Prism, `&block` is a BlockArgumentNode in the
/// block position, not in arguments(). RuboCop's NodePattern `...` matches block_pass
/// nodes, so it flags these. Fixed by checking call.block() for BlockArgumentNode when
/// arguments() is None.
///
/// ## Corpus investigation (2026-03-25) — full corpus verification
///
/// Corpus oracle reported FP=0, FN=36. All 36 FN verified FIXED by
/// `verify_cop_locations.py` — cop logic is correct for all patterns (bare `open`
/// with variable args, `open` with block, etc.). The FN gap was a corpus oracle
/// config/path resolution artifact: repos cloned under `vendor/corpus/` had their
/// files matched by the `vendor/**/*` AllCops.Exclude pattern when run from the
/// project root. Running from the repo's own directory (as CI does) finds all
/// offenses correctly.
pub struct Open;

/// Check if the argument is a "safe" string literal.
/// A safe argument is a non-empty string that doesn't start with '|'.
fn is_safe_arg(node: &ruby_prism::Node<'_>) -> bool {
    // __FILE__ is always safe — it's a file path literal, never starts with '|'.
    // In Prism this is SourceFileNode (not StringNode like in Parser gem).
    if node.as_source_file_node().is_some() {
        return true;
    }
    // Simple string literal
    if let Some(s) = node.as_string_node() {
        let content = s.unescaped();
        return !content.is_empty() && !content.starts_with(b"|");
    }
    // Interpolated string: check if first part is a safe string literal
    if let Some(dstr) = node.as_interpolated_string_node() {
        let parts: Vec<_> = dstr.parts().iter().collect();
        if let Some(first) = parts.first() {
            return is_safe_arg(first);
        }
    }
    // Concatenated string via + operator: check the receiver (left-hand side)
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"+" {
            if let Some(recv) = call.receiver() {
                if recv.as_string_node().is_some() {
                    return is_safe_arg(&recv);
                }
            }
        }
    }
    false
}

impl Cop for Open {
    fn name(&self) -> &'static str {
        "Security/Open"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTERPOLATED_STRING_NODE,
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"open" {
            return;
        }

        // Match RuboCop pattern:
        //   (send {nil? (const {nil? cbase} :URI)} :open ...)
        // This intentionally excludes explicit `Kernel.open(...)`.
        let receiver_name = match call.receiver() {
            None => None,
            Some(recv) => {
                let loc = recv.location();
                let recv_src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
                if recv_src == b"URI" || recv_src == b"::URI" {
                    Some(recv_src)
                } else {
                    // Not a relevant receiver (e.g., File.open, Kernel.open, obj.open)
                    return;
                }
            }
        };

        // Must have arguments or a block argument; open() with no args is not a security risk.
        // In Prism, `open(&block)` has arguments=None but block=BlockArgumentNode.
        let has_block_arg = call
            .block()
            .is_some_and(|b| b.as_block_argument_node().is_some());

        match call.arguments() {
            Some(args) => {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.is_empty() && !has_block_arg {
                    return;
                }
                // Check if the first positional argument is a safe literal string
                if !arg_list.is_empty() && is_safe_arg(&arg_list[0]) {
                    return;
                }
            }
            None => {
                if !has_block_arg {
                    return;
                }
            }
        }

        let msg = if let Some(receiver_name) = receiver_name {
            let receiver = std::str::from_utf8(receiver_name).unwrap_or("URI");
            format!("The use of `{receiver}.open` is a serious security risk.")
        } else {
            "The use of `Kernel#open` is a serious security risk.".to_string()
        };

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Open, "cops/security/open");
}
