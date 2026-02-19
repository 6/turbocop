use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};

pub struct FindById;

/// Check if a call has exactly one keyword argument with key `:id`.
fn has_sole_id_keyword_arg(call: &ruby_prism::CallNode<'_>) -> bool {
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };
    let all_args: Vec<_> = args.arguments().iter().collect();
    if all_args.len() != 1 {
        return false;
    }
    let kw = match all_args[0].as_keyword_hash_node() {
        Some(k) => k,
        None => return false,
    };
    let elements: Vec<_> = kw.elements().iter().collect();
    if elements.len() != 1 {
        return false;
    }
    let assoc = match elements[0].as_assoc_node() {
        Some(a) => a,
        None => return false,
    };
    let sym = match assoc.key().as_symbol_node() {
        Some(s) => s,
        None => return false,
    };
    sym.unescaped() == b"id"
}

impl Cop for FindById {
    fn name(&self) -> &'static str {
        "Rails/FindById"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();

        // Pattern 1: find_by_id!(id)
        if name == b"find_by_id!" {
            if call.receiver().is_some() && call.arguments().is_some() {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `find` instead of `find_by_id!`.".to_string(),
                )];
            }
            return Vec::new();
        }

        // Pattern 2: find_by!(id: value) — only when id is the sole argument
        if name == b"find_by!" {
            if call.receiver().is_some() && has_sole_id_keyword_arg(&call) {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `find` instead of `find_by!`.".to_string(),
                )];
            }
            return Vec::new();
        }

        // Pattern 3: where(id: value).take!
        if name == b"take!" {
            let chain = match util::as_method_chain(node) {
                Some(c) => c,
                None => return Vec::new(),
            };
            if chain.inner_method != b"where" {
                return Vec::new();
            }
            // Check that `where` has `id:` as the sole keyword arg
            if has_sole_id_keyword_arg(&chain.inner_call) {
                let loc = chain.inner_call.message_loc().unwrap_or(chain.inner_call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `find` instead of `where(id: ...).take!`.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FindById, "cops/rails/find_by_id");
}
