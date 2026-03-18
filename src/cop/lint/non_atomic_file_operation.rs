use crate::cop::node_type::{CONSTANT_PATH_NODE, CONSTANT_READ_NODE, IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for non-atomic file operations where an existence check precedes
/// a file create/remove. RuboCop fires two offenses per pattern:
///
/// 1. On the existence check condition: "Remove unnecessary existence check `X.exist?`."
/// 2. On the file operation call (only for non-force methods): "Use atomic file
///    operation method `FileUtils.replacement`."
///
/// ## Root cause analysis (23.4% match rate → rewrite)
///
/// Previous implementation had multiple gaps causing 621 FN and 11 FP:
/// - Only emitted the "Use atomic" offense, missing the "Remove unnecessary" offense entirely
/// - Did not handle force methods (makedirs, mkdir_p, mkpath, rm_f, rm_rf) which only get
///   the existence check offense
/// - Did not handle negated `if !File.exist?` conditions
/// - Did not handle `elsif` form
/// - Did not check `force: false` to skip the offense
/// - Did not verify path arguments match between exist check and file operation
/// - Did not check condition is not compound (&&/||)
/// - Body could have multiple statements; RuboCop requires the file op's parent to be
///   the if node directly (implying single-statement body)
///
/// Rewritten to match RuboCop's behavior: start from if/unless nodes, extract the
/// existence check from condition, verify single file-op body, emit both offenses.
///
/// ## Fix: accept any constant receiver for file operations (407 FN reduction)
///
/// Previous implementation restricted file operation receivers to `FileUtils` and `Dir` only.
/// RuboCop accepts ANY constant receiver (`node.receiver&.const_type?`), so patterns like
/// `File.delete(path) if File.exist?(path)` and `File.unlink(path) if File.exist?(path)`
/// were missed. The fix relaxes the receiver check to accept any constant, matching RuboCop.
pub struct NonAtomicFileOperation;

const MAKE_METHODS: &[&[u8]] = &[b"mkdir"];
const MAKE_FORCE_METHODS: &[&[u8]] = &[b"makedirs", b"mkdir_p", b"mkpath"];
const REMOVE_METHODS: &[&[u8]] = &[
    b"remove",
    b"delete",
    b"unlink",
    b"remove_file",
    b"rm",
    b"rmdir",
    b"safe_unlink",
];
const RECURSIVE_REMOVE_METHODS: &[&[u8]] =
    &[b"remove_dir", b"remove_entry", b"remove_entry_secure"];
const REMOVE_FORCE_METHODS: &[&[u8]] = &[b"rm_f", b"rm_rf"];

const EXIST_METHODS: &[&[u8]] = &[b"exist?", b"exists?"];
const EXIST_CLASSES: &[&[u8]] = &[b"FileTest", b"File", b"Dir", b"Shell"];

/// All recognized file operation methods (force + non-force).
fn is_file_op_method(method: &[u8]) -> bool {
    MAKE_METHODS.contains(&method)
        || MAKE_FORCE_METHODS.contains(&method)
        || REMOVE_METHODS.contains(&method)
        || RECURSIVE_REMOVE_METHODS.contains(&method)
        || REMOVE_FORCE_METHODS.contains(&method)
}

/// Whether the method is a "force" variant that doesn't need a replacement suggestion.
fn is_force_method(method: &[u8]) -> bool {
    MAKE_FORCE_METHODS.contains(&method) || REMOVE_FORCE_METHODS.contains(&method)
}

/// Get the replacement method name for non-force methods.
fn replacement_method(method: &[u8]) -> &'static str {
    if MAKE_METHODS.contains(&method) {
        "mkdir_p"
    } else if REMOVE_METHODS.contains(&method) {
        "rm_f"
    } else if RECURSIVE_REMOVE_METHODS.contains(&method) {
        "rm_rf"
    } else {
        // Should not reach here for non-force methods
        "rm_f"
    }
}

/// Extract the constant class name from a receiver node (handles both ConstantReadNode
/// and ConstantPathNode for `::File` etc.).
fn const_name<'a>(node: &'a ruby_prism::Node<'a>) -> Option<&'a [u8]> {
    if let Some(cr) = node.as_constant_read_node() {
        Some(cr.name().as_slice())
    } else if let Some(cp) = node.as_constant_path_node() {
        cp.name().map(|n| n.as_slice())
    } else {
        None
    }
}

/// Check if a call node is an exist? call on a recognized class.
fn is_exist_call(call: &ruby_prism::CallNode<'_>) -> bool {
    if !EXIST_METHODS.contains(&call.name().as_slice()) {
        return false;
    }
    if let Some(recv) = call.receiver() {
        if let Some(name) = const_name(&recv) {
            return EXIST_CLASSES.contains(&name);
        }
    }
    false
}

/// Check if the condition contains an exist? call (direct, negated, or parenthesized).
/// Returns true if found, and provides the exist call's first argument source and
/// receiver/method info for the diagnostic message.
fn find_exist_info(condition: &ruby_prism::Node<'_>) -> Option<ExistInfo> {
    if let Some(call) = condition.as_call_node() {
        if call.name().as_slice() == b"!" {
            // Negated: `!File.exist?(path)`
            if let Some(inner) = call.receiver() {
                if let Some(inner_call) = inner.as_call_node() {
                    if is_exist_call(&inner_call) {
                        let first_arg = inner_call
                            .arguments()
                            .and_then(|args| args.arguments().iter().next())
                            .map(|a| a.location().as_slice().to_vec());
                        let recv_name = if let Some(recv) = inner_call.receiver() {
                            const_name(&recv).unwrap_or(b"File").to_vec()
                        } else {
                            b"File".to_vec()
                        };
                        let method_name = inner_call.name().as_slice().to_vec();
                        return Some(ExistInfo {
                            first_arg,
                            recv_name,
                            method_name,
                        });
                    }
                }
            }
            return None;
        }
        if is_exist_call(&call) {
            let first_arg = call
                .arguments()
                .and_then(|args| args.arguments().iter().next())
                .map(|a| a.location().as_slice().to_vec());
            let recv_name = if let Some(recv) = call.receiver() {
                const_name(&recv).unwrap_or(b"File").to_vec()
            } else {
                b"File".to_vec()
            };
            let method_name = call.name().as_slice().to_vec();
            return Some(ExistInfo {
                first_arg,
                recv_name,
                method_name,
            });
        }
    }
    None
}

struct ExistInfo {
    first_arg: Option<Vec<u8>>,
    recv_name: Vec<u8>,
    method_name: Vec<u8>,
}

/// Check if a call node has `force: false` in its arguments.
fn has_explicit_not_force(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if check_force_false(&arg) {
                return true;
            }
        }
    }
    false
}

/// Check a node (keyword hash or hash) for `force: false`.
fn check_force_false(node: &ruby_prism::Node<'_>) -> bool {
    let elements = if let Some(kw_hash) = node.as_keyword_hash_node() {
        kw_hash.elements()
    } else if let Some(hash) = node.as_hash_node() {
        hash.elements()
    } else {
        return false;
    };

    for elem in elements.iter() {
        if let Some(assoc) = elem.as_assoc_node() {
            if let Some(key) = assoc.key().as_symbol_node() {
                if key.unescaped() == b"force" && assoc.value().as_false_node().is_some() {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a condition is a compound expression (&&/||).
fn is_operator_condition(condition: &ruby_prism::Node<'_>) -> bool {
    condition.as_and_node().is_some() || condition.as_or_node().is_some()
}

impl Cop for NonAtomicFileOperation {
    fn name(&self) -> &'static str {
        "Lint/NonAtomicFileOperation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_NODE, CONSTANT_READ_NODE, IF_NODE, UNLESS_NODE]
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
        // Extract condition, body, and else-branch presence from if/unless nodes
        let (condition, body, has_else) = if let Some(if_node) = node.as_if_node() {
            (
                if_node.predicate(),
                if_node.statements(),
                if_node.subsequent().is_some(),
            )
        } else if let Some(unless_node) = node.as_unless_node() {
            (
                unless_node.predicate(),
                unless_node.statements(),
                unless_node.else_clause().is_some(),
            )
        } else {
            return;
        };

        // Skip if there's an else/elsif branch
        if has_else {
            return;
        }

        // Skip compound conditions (&&, ||)
        if is_operator_condition(&condition) {
            return;
        }

        // Extract existence check info from the condition
        let exist_info = match find_exist_info(&condition) {
            Some(info) => info,
            None => return,
        };

        // Check body has exactly one statement that is a file operation
        let body_stmts = match body {
            Some(s) => s,
            None => return,
        };

        let stmts: Vec<_> = body_stmts.body().iter().collect();
        if stmts.len() != 1 {
            return;
        }

        let file_op_call = match stmts[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = file_op_call.name().as_slice();
        if !is_file_op_method(method) {
            return;
        }

        // Check receiver is any constant (RuboCop: node.receiver&.const_type?)
        let recv = match file_op_call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Must be a constant receiver (ConstantReadNode or ConstantPathNode)
        if const_name(&recv).is_none() {
            return;
        }

        // Check explicit `force: false` — skip entirely
        if has_explicit_not_force(&file_op_call) {
            return;
        }

        // Check first arguments match
        let op_first_arg = file_op_call
            .arguments()
            .and_then(|args| args.arguments().iter().next())
            .map(|a| a.location().as_slice().to_vec());
        if exist_info.first_arg != op_first_arg {
            return;
        }

        // Emit offense on file operation (only for non-force methods)
        if !is_force_method(method) {
            let replacement = replacement_method(method);
            let loc = file_op_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Use atomic file operation method `FileUtils.{replacement}`."),
            ));
        }

        // Emit offense on the existence check condition
        // Range: from the if/unless keyword through the end of the condition
        let keyword_start = if let Some(if_node) = node.as_if_node() {
            if let Some(loc) = if_node.if_keyword_loc() {
                loc.start_offset()
            } else {
                return;
            }
        } else if let Some(unless_node) = node.as_unless_node() {
            unless_node.keyword_loc().start_offset()
        } else {
            return;
        };

        let (line, column) = source.offset_to_line_col(keyword_start);

        let recv_str = std::str::from_utf8(&exist_info.recv_name).unwrap_or("File");
        let method_str = std::str::from_utf8(&exist_info.method_name).unwrap_or("exist?");

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Remove unnecessary existence check `{recv_str}.{method_str}`."),
        ));
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
