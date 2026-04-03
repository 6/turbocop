use crate::cop::shared::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-20) — extended corpus
///
/// Extended corpus oracle reported FP=2, FN=1.
///
/// FP=2: Fixed by skipping `JSON.load` calls that include a `create_additions`
/// keyword argument (in any position — direct kwarg, hash arg, or after proc arg).
/// When `create_additions` is explicit, the call is safe regardless of its value.
///
/// FN=1: Fixed by also detecting `JSON.restore` (alias for `JSON.load`).
/// Message uses "JSON.restore" instead of "JSON.load" to match RuboCop.
pub struct JsonLoad;

impl Cop for JsonLoad {
    fn name(&self) -> &'static str {
        "Security/JSONLoad"
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

        let method_name = call.name().as_slice();
        if method_name != b"load" && method_name != b"restore" {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_json = is_constant_named(source, &recv, b"JSON");
        if !is_json {
            return;
        }

        // Skip if any argument contains a `create_additions` keyword
        if has_create_additions_kwarg(&call) {
            return;
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("load");
        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `JSON.parse` over `JSON.{method_str}`."),
        ));
    }
}

/// Check if a hash-like node (KeywordHashNode or HashNode) contains `create_additions` key.
fn hash_has_create_additions(node: &ruby_prism::Node<'_>) -> bool {
    let elements = if let Some(kh) = node.as_keyword_hash_node() {
        kh.elements()
    } else if let Some(h) = node.as_hash_node() {
        h.elements()
    } else {
        return false;
    };
    for elem in elements.iter() {
        if let Some(assoc) = elem.as_assoc_node() {
            if let Some(sym) = assoc.key().as_symbol_node() {
                if sym.unescaped() == b"create_additions" {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if any argument to JSON.load/restore contains a `create_additions` keyword.
fn has_create_additions_kwarg(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if hash_has_create_additions(&arg) {
                return true;
            }
        }
    }
    false
}

fn is_constant_named(_source: &SourceFile, node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == name;
    }
    if let Some(cp) = node.as_constant_path_node() {
        // Check if the last segment is the target name
        if let Some(child) = cp.name() {
            if child.as_slice() == name {
                // For ::JSON, parent is None; for Foo::JSON, parent is Some
                return cp.parent().is_none();
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(JsonLoad, "cops/security/json_load");
}
