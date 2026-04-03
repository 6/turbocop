use crate::cop::shared::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle reported FP=0, FN=1.
///
/// FN=1: In a tracked `.irbrc` dotfile that was skipped by discovery. Fixed in
/// `src/fs.rs` by merging `git ls-files` tracked Ruby files into discovery results,
/// which makes tracked hidden files visible to cops while preserving normal
/// directory walking behavior.
///
/// ## Corpus investigation (2026-03-25) — full corpus verification
///
/// Corpus oracle reported FP=0, FN=2. All 2 FN verified FIXED by
/// `verify_cop_locations.py`. Cop logic handles all `IO.read/write/
/// binread/binwrite/foreach/readlines` patterns correctly. The FN gap
/// was a corpus oracle config/path resolution artifact.
pub struct IoMethods;

const DANGEROUS_METHODS: &[&[u8]] = &[
    b"read",
    b"write",
    b"binread",
    b"binwrite",
    b"foreach",
    b"readlines",
];

fn first_arg_starts_with_pipe(node: &ruby_prism::Node<'_>) -> bool {
    let Some(string) = node.as_string_node() else {
        return false;
    };

    let bytes = string.unescaped();
    let trimmed = match std::str::from_utf8(bytes) {
        Ok(text) => text.trim().as_bytes(),
        Err(_) => {
            let start = bytes
                .iter()
                .position(|b| !b.is_ascii_whitespace())
                .unwrap_or(bytes.len());
            &bytes[start..]
        }
    };

    trimmed.starts_with(b"|")
}

impl Cop for IoMethods {
    fn name(&self) -> &'static str {
        "Security/IoMethods"
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
        if !DANGEROUS_METHODS.contains(&method) {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // RuboCop only flags when the receiver source is exactly `IO`, not `::IO`.
        // Intentionally do NOT match constant_path_node (::IO) — RuboCop's pattern is
        // `receiver.source == 'IO'` which doesn't match the `::IO` qualified form.
        let is_io = if let Some(cr) = recv.as_constant_read_node() {
            cr.name().as_slice() == b"IO"
        } else {
            false
        };

        if !is_io {
            return;
        }

        if let Some(args) = call.arguments() {
            if let Some(first_arg) = args.arguments().iter().next() {
                if first_arg_starts_with_pipe(&first_arg) {
                    return;
                }
            }
        }

        let method_str = std::str::from_utf8(method).unwrap_or("");
        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("The use of `IO.{method_str}` is a security risk."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(IoMethods, "cops/security/io_methods");
}
