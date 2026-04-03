use crate::cop::shared::node_type::{
    ARRAY_NODE, ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, STRING_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DotSeparatedKeys;

impl Cop for DotSeparatedKeys {
    fn name(&self) -> &'static str {
        "Rails/DotSeparatedKeys"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            ASSOC_NODE,
            CALL_NODE,
            KEYWORD_HASH_NODE,
            STRING_NODE,
            SYMBOL_NODE,
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

        let method_name = call.name().as_slice();
        if method_name != b"t" && method_name != b"translate" {
            return;
        }

        // Receiver can be I18n, ::I18n, or absent (Rails helper `t`)
        // RuboCop pattern: (const {nil? cbase} :I18n) — only plain I18n or ::I18n,
        // NOT namespaced like Formtastic::I18n
        if let Some(recv) = call.receiver() {
            if let Some(cr) = recv.as_constant_read_node() {
                // Plain `I18n`
                if cr.name().as_slice() != b"I18n" {
                    return;
                }
            } else if let Some(cp) = recv.as_constant_path_node() {
                // `::I18n` — parent must be None (cbase) and name must be I18n
                if cp.parent().is_some() {
                    return; // Namespaced like Formtastic::I18n — skip
                }
                if cp.name().map(|n| n.as_slice()) != Some(b"I18n") {
                    return;
                }
            } else {
                return; // Not a constant receiver
            }
        }

        // Look for a `scope:` keyword argument — this cop flags scope-based keys
        // and suggests using dot-separated string keys instead.
        // Only flag when scope value is an array (of literals) or a symbol.
        // String scope values are already dot-separated notation.
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // RuboCop requires the first positional argument to be a symbol or string literal.
        // If the first arg is a variable, method call, array, etc., don't flag.
        let arg_list = args.arguments();
        let first_arg = match arg_list.iter().next() {
            Some(a) => a,
            None => return,
        };
        if first_arg.as_symbol_node().is_none() && first_arg.as_string_node().is_none() {
            return;
        }

        for arg in arg_list.iter() {
            let hash = if let Some(h) = arg.as_keyword_hash_node() {
                h.elements()
            } else {
                continue;
            };
            for elem in hash.iter() {
                let assoc = match elem.as_assoc_node() {
                    Some(a) => a,
                    None => continue,
                };
                let is_scope_key = if let Some(sym) = assoc.key().as_symbol_node() {
                    sym.unescaped() == b"scope"
                } else {
                    false
                };
                if is_scope_key {
                    let value = assoc.value();
                    // Only flag when scope is a symbol or an array of all literals
                    if value.as_symbol_node().is_some() {
                        // scope: :invitation — should be dot-separated
                    } else if let Some(array) = value.as_array_node() {
                        // scope: [:foo, :bar] — only flag if all elements are literals
                        let all_literals = array
                            .elements()
                            .iter()
                            .all(|e| e.as_symbol_node().is_some() || e.as_string_node().is_some());
                        if !all_literals {
                            continue;
                        }
                    } else {
                        // scope: 'string' or scope: variable — don't flag
                        continue;
                    }

                    let loc = assoc.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use dot-separated keys instead of the `:scope` option.".to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DotSeparatedKeys, "cops/rails/dot_separated_keys");
}
