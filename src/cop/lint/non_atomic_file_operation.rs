use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, IF_NODE, UNLESS_NODE};

pub struct NonAtomicFileOperation;

const MAKE_METHODS: &[&[u8]] = &[b"mkdir"];
const REMOVE_METHODS: &[&[u8]] = &[
    b"remove", b"delete", b"unlink", b"remove_file", b"rm", b"rmdir", b"safe_unlink",
];
const RECURSIVE_REMOVE_METHODS: &[&[u8]] = &[
    b"remove_dir", b"remove_entry", b"remove_entry_secure",
];
const EXIST_METHODS: &[&[u8]] = &[b"exist?", b"exists?"];
const EXIST_CLASSES: &[&[u8]] = &[b"FileTest", b"File", b"Dir", b"Shell"];
const FS_CLASSES: &[&[u8]] = &[b"FileUtils", b"Dir"];

impl Cop for NonAtomicFileOperation {
    fn name(&self) -> &'static str {
        "Lint/NonAtomicFileOperation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, IF_NODE, UNLESS_NODE]
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
        // Look for if/unless nodes
        let (condition, body, has_else) = if let Some(if_node) = node.as_if_node() {
            (if_node.predicate(), if_node.statements(), if_node.subsequent().is_some())
        } else if let Some(unless_node) = node.as_unless_node() {
            (unless_node.predicate(), unless_node.statements(), unless_node.else_clause().is_some())
        } else {
            return;
        };

        // Skip if there's an else branch
        if has_else {
            return;
        }

        // Check if condition is a File.exist?/Dir.exist? call
        let condition_call = match condition.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let cond_method = condition_call.name().as_slice();
        if !EXIST_METHODS.iter().any(|m| *m == cond_method) {
            return;
        }

        // Check receiver is File/Dir/FileTest/Shell
        let cond_recv = match condition_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_exist_class = if let Some(cr) = cond_recv.as_constant_read_node() {
            EXIST_CLASSES.iter().any(|c| *c == cr.name().as_slice())
        } else if let Some(cp) = cond_recv.as_constant_path_node() {
            cp.name().is_some_and(|n| EXIST_CLASSES.iter().any(|c| *c == n.as_slice()))
        } else {
            false
        };

        if !is_exist_class {
            return;
        }

        // Check body contains a file operation
        let body_stmts = match body {
            Some(s) => s,
            None => return,
        };

        let stmts: Vec<_> = body_stmts.body().iter().collect();
        if stmts.is_empty() {
            return;
        }

        for stmt in &stmts {
            if let Some(call) = stmt.as_call_node() {
                let method = call.name().as_slice();

                let is_file_op = MAKE_METHODS.iter().any(|m| *m == method)
                    || REMOVE_METHODS.iter().any(|m| *m == method)
                    || RECURSIVE_REMOVE_METHODS.iter().any(|m| *m == method);

                if !is_file_op {
                    continue;
                }

                // Check receiver is FileUtils/Dir
                let recv = match call.receiver() {
                    Some(r) => r,
                    None => continue,
                };

                let is_fs_class = if let Some(cr) = recv.as_constant_read_node() {
                    FS_CLASSES.iter().any(|c| *c == cr.name().as_slice())
                } else if let Some(cp) = recv.as_constant_path_node() {
                    cp.name().is_some_and(|n| FS_CLASSES.iter().any(|c| *c == n.as_slice()))
                } else {
                    false
                };

                if !is_fs_class {
                    continue;
                }

                let replacement = if MAKE_METHODS.iter().any(|m| *m == method) {
                    "mkdir_p"
                } else if REMOVE_METHODS.iter().any(|m| *m == method) {
                    "rm_f"
                } else {
                    "rm_rf"
                };

                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Use atomic file operation method `FileUtils.{replacement}`."
                    ),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NonAtomicFileOperation,
        "cops/lint/non_atomic_file_operation"
    );
}
