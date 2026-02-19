use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, FALSE_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE, TRUE_NODE};

pub struct RedundantAllowNil;

/// Find a keyword pair (key + value) by key name in a call's arguments.
/// Returns (pair_start_offset, value_node).
fn find_keyword_pair<'a>(
    call: &ruby_prism::CallNode<'a>,
    key: &[u8],
) -> Option<(usize, ruby_prism::Node<'a>)> {
    let args = call.arguments()?;
    for arg in args.arguments().iter() {
        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if sym.unescaped() == key {
                            return Some((assoc.key().location().start_offset(), assoc.value()));
                        }
                    }
                }
            }
        }
    }
    None
}

impl Cop for RedundantAllowNil {
    fn name(&self) -> &'static str {
        "Rails/RedundantAllowNil"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, FALSE_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE, TRUE_NODE]
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
        if name != b"validates" && name != b"validates!" {
            return Vec::new();
        }
        if call.receiver().is_some() {
            return Vec::new();
        }

        let (nil_offset, allow_nil_val) = match find_keyword_pair(&call, b"allow_nil") {
            Some(v) => v,
            None => return Vec::new(),
        };
        let (_blank_offset, allow_blank_val) = match find_keyword_pair(&call, b"allow_blank") {
            Some(v) => v,
            None => return Vec::new(),
        };

        // Compare boolean values
        let nil_is_true = is_true_literal(&allow_nil_val);
        let nil_is_false = is_false_literal(&allow_nil_val);
        let blank_is_true = is_true_literal(&allow_blank_val);
        let blank_is_false = is_false_literal(&allow_blank_val);

        let msg = if (nil_is_true && blank_is_true) || (nil_is_false && blank_is_false) {
            "`allow_nil` is redundant when `allow_blank` has the same value."
        } else if nil_is_false && blank_is_true {
            "`allow_nil: false` is redundant when `allow_blank` is true."
        } else {
            return Vec::new();
        };

        let (line, column) = source.offset_to_line_col(nil_offset);
        vec![self.diagnostic(source, line, column, msg.to_string())]
    }
}

fn is_true_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_true_node().is_some()
}

fn is_false_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_false_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantAllowNil, "cops/rails/redundant_allow_nil");
}
