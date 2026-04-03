use crate::cop::shared::node_type::{
    ARRAY_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE, KEYWORD_HASH_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for `Hash` creation with a mutable default value.
/// `Hash.new([])` or `Hash.new({})` shares the default across all keys.
///
/// Corpus FN=10 fix: keyword arguments like `Hash.new(unknown: true)` produce
/// a `KeywordHashNode` in Prism (not `HashNode`). Added detection for keyword
/// hash args as mutable defaults. Also added exclusion for `Hash.new(capacity: N)`
/// which is a legitimate non-mutable argument per RuboCop's pattern.
///
/// Corpus FN=4 fix: `Hash.new(unknown: true) { 0 }` was missed because the
/// block early-return skipped argument analysis. RuboCop's pattern does not
/// exclude calls with blocks — it flags mutable arguments regardless. Removed
/// the block early-return; `Hash.new { ... }` with no mutable argument still
/// passes because `call.arguments()` is None.
///
/// Corpus FP=1 fix: `Hash.new(Hash.new(0))` was flagged because `is_mutable_value`
/// treated any `Array.new(...)` or `Hash.new(...)` call as mutable regardless of
/// arguments. RuboCop's pattern only matches the no-argument form (`Array.new` /
/// `Hash.new`), since passing an argument (e.g., `Hash.new(0)`) produces a hash
/// with an immutable default. Added `call.arguments().is_none()` guard.
pub struct SharedMutableDefault;

impl Cop for SharedMutableDefault {
    fn name(&self) -> &'static str {
        "Lint/SharedMutableDefault"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
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

        if call.name().as_slice() != b"new" {
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
            cp.parent().is_none() && cp.name().map(|n| n.as_slice() == b"Hash").unwrap_or(false)
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
    if node.as_hash_node().is_some() {
        return true;
    }
    // Keyword hash args like Hash.new(unknown: true) — Prism wraps these in KeywordHashNode
    if let Some(kh) = node.as_keyword_hash_node() {
        return !is_capacity_keyword_argument(&kh);
    }
    // Array.new or Hash.new with no arguments (only bare or root-qualified, not Concurrent::Array.new)
    // Hash.new(0) or Array.new(5) are not mutable defaults — RuboCop only flags the no-arg form.
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"new" && call.arguments().is_none() {
            if let Some(recv) = call.receiver() {
                let is_plain_array_or_hash = if let Some(cr) = recv.as_constant_read_node() {
                    let name = cr.name().as_slice();
                    name == b"Array" || name == b"Hash"
                } else if let Some(cp) = recv.as_constant_path_node() {
                    cp.parent().is_none()
                        && cp
                            .name()
                            .map(|n| n.as_slice() == b"Array" || n.as_slice() == b"Hash")
                            .unwrap_or(false)
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

/// Returns true if the keyword hash node is a single `capacity:` keyword argument,
/// which is a legitimate Hash.new argument (not a mutable default).
fn is_capacity_keyword_argument(kh: &ruby_prism::KeywordHashNode<'_>) -> bool {
    let elements: Vec<_> = kh.elements().iter().collect();
    if elements.len() == 1 {
        if let Some(pair) = elements[0].as_assoc_node() {
            if let Some(sym) = pair.key().as_symbol_node() {
                return sym.unescaped() == b"capacity";
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
