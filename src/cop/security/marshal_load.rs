use crate::cop::shared::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle reported FP=7, FN=3.
///
/// FP=7: Fixed by checking argument count — RuboCop's NodePattern matches exactly
/// one argument, so `Marshal.load(data, filter)` (2+ args) is not flagged. Fix in
/// commit fe41845.
///
/// FN=3: All in `rubyworks__facets__12326d4` at `work/sandbox/multiton2.rb` (lines
/// 68, 210, 460). These files are tracked but under a `.gitignore`d directory.
/// Fixed in `src/fs.rs` by merging `git ls-files` tracked Ruby files into discovery,
/// so tracked ignored files are linted like RuboCop.
///
/// ## Corpus investigation (2026-03-25) — full corpus verification
///
/// Corpus oracle reported FP=0, FN=13. All 13 FN verified FIXED by
/// `verify_cop_locations.py`. Cop logic handles all `Marshal.load` and
/// `Marshal.restore` patterns correctly. The FN gap was a corpus oracle
/// config/path resolution artifact (same as Security/Open).
pub struct MarshalLoad;

impl Cop for MarshalLoad {
    fn name(&self) -> &'static str {
        "Security/MarshalLoad"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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

        let method = call.name().as_slice();
        if method != b"load" && method != b"restore" {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !is_top_level_marshal(&recv, source) {
            return;
        }

        let args = match call.arguments() {
            Some(args) => args,
            None => return,
        };
        let mut arg_iter = args.arguments().iter();
        let Some(first_arg) = arg_iter.next() else {
            return;
        };

        // RuboCop's pattern matches exactly one argument — Marshal.load(data, filter)
        // with a second arg (proc filter, freeze: true, etc.) is not flagged.
        if arg_iter.next().is_some() {
            return;
        }

        // Exclude the "deep copy hack" pattern: Marshal.load(Marshal.dump(...))
        if let Some(inner_call) = first_arg.as_call_node() {
            if inner_call.name().as_slice() == b"dump" {
                if let Some(inner_recv) = inner_call.receiver() {
                    if is_top_level_marshal(&inner_recv, source) {
                        return;
                    }
                }
            }
        }

        let method_name = if method == b"restore" {
            "Marshal.restore"
        } else {
            "Marshal.load"
        };

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Avoid using `{method_name}`."),
        ));
    }
}

fn is_top_level_marshal(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == b"Marshal";
    }
    if let Some(cp) = node.as_constant_path_node() {
        let loc = cp.location();
        let recv_src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        return recv_src == b"Marshal" || recv_src == b"::Marshal";
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(MarshalLoad, "cops/security/marshal_load");
}
