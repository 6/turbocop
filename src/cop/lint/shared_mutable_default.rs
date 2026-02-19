use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE, KEYWORD_HASH_NODE};

/// Checks for `Hash` creation with a mutable default value.
/// `Hash.new([])` or `Hash.new({})` shares the default across all keys.
pub struct SharedMutableDefault;

impl Cop for SharedMutableDefault {
    fn name(&self) -> &'static str {
        "Lint/SharedMutableDefault"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE, KEYWORD_HASH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"new" {
            return;
        }

        // Must not have a block (Hash.new { ... } is fine)
        if call.block().is_some() {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Must be bare `Hash` or root `::Hash`, not qualified like `Concurrent::Hash`
        let is_plain_hash = if let Some(cr) = receiver.as_constant_read_node() {
            cr.name().as_slice() == b"Hash"
        } else if let Some(cp) = receiver.as_constant_path_node() {
            // ::Hash (cbase) — parent is None
            cp.parent().is_none()
                && cp.name().map(|n| n.as_slice() == b"Hash").unwrap_or(false)
        } else {
            false
        };

        if !is_plain_hash {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args: Vec<_> = arguments.arguments().iter().collect();
        if args.is_empty() {
            return;
        }

        let first_arg = &args[0];

        // Check for mutable defaults: [], {}, Array.new, Hash.new
        let is_mutable = is_mutable_value(first_arg);

        if !is_mutable {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not create a Hash with a mutable default value as the default value can accidentally be changed.".to_string(),
        ));
    }
}

fn is_mutable_value(node: &ruby_prism::Node<'_>) -> bool {
    // Array literal []
    if node.as_array_node().is_some() {
        return true;
    }
    // Hash literal {}
    // Note: as_keyword_hash_node() is not checked here because keyword hash
    // nodes (keyword args like `foo(a: 1)`) cannot appear as Hash.new arguments.
    if node.as_hash_node().is_some() {
        return true;
    }
    // Array.new or Hash.new (only bare or root-qualified, not Concurrent::Array.new)
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"new" {
            if let Some(recv) = call.receiver() {
                let is_plain_array_or_hash = if let Some(cr) = recv.as_constant_read_node() {
                    let name = cr.name().as_slice();
                    name == b"Array" || name == b"Hash"
                } else if let Some(cp) = recv.as_constant_path_node() {
                    cp.parent().is_none()
                        && cp.name().map(|n| {
                            n.as_slice() == b"Array" || n.as_slice() == b"Hash"
                        }).unwrap_or(false)
                } else {
                    false
                };
                if is_plain_array_or_hash {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SharedMutableDefault, "cops/lint/shared_mutable_default");
}
