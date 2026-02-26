use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Check if a call has an explicit block, block_pass (&:sym), or an implicit
/// block via a final symbol argument for methods in MethodsAcceptingSymbol.
fn has_block_or_implicit_block(
    call: &ruby_prism::CallNode<'_>,
    method_name: &str,
    methods_accepting_symbol: &[String],
) -> bool {
    // Check for explicit block ({ ... }, do...end) or block_pass (&:sym)
    if let Some(block) = call.block() {
        if block.as_block_node().is_some() || block.as_block_argument_node().is_some() {
            return true;
        }
    }

    // Check for implicit block: final symbol arg for MethodsAcceptingSymbol methods
    if methods_accepting_symbol.iter().any(|m| m == method_name) {
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if let Some(last) = arg_list.last() {
                if last.as_symbol_node().is_some() {
                    return true;
                }
            }
        }
    }

    false
}

pub struct CollectionMethods;

impl Cop for CollectionMethods {
    fn name(&self) -> &'static str {
        "Style/CollectionMethods"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let preferred_methods = config
            .get_string_hash("PreferredMethods")
            .unwrap_or_else(|| {
                // Default preferred methods per RuboCop's default.yml
                let mut m = std::collections::HashMap::new();
                m.insert("collect".to_string(), "map".to_string());
                m.insert("collect!".to_string(), "map!".to_string());
                m.insert("collect_concat".to_string(), "flat_map".to_string());
                m.insert("inject".to_string(), "reduce".to_string());
                m.insert("detect".to_string(), "find".to_string());
                m.insert("find_all".to_string(), "select".to_string());
                m.insert("member?".to_string(), "include?".to_string());
                m
            });
        let methods_accepting_symbol = config
            .get_string_array("MethodsAcceptingSymbol")
            .unwrap_or_else(|| vec!["inject".to_string(), "reduce".to_string()]);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must have a receiver (collection.method)
        if call.receiver().is_none() {
            return;
        }

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");

        if let Some(preferred) = preferred_methods.get(method_name) {
            // RuboCop only flags calls that have a block or implicit block:
            // 1. Explicit block: items.collect { |e| ... } or items.collect do...end
            // 2. Block pass: items.collect(&:to_s)
            // 3. Symbol arg for MethodsAcceptingSymbol methods: items.inject(:+)
            // Plain calls without blocks (e.g., list.member?(x)) are NOT flagged.
            if !has_block_or_implicit_block(&call, method_name, &methods_accepting_symbol) {
                return;
            }

            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Prefer `{}` over `{}`.", preferred, method_name),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CollectionMethods, "cops/style/collection_methods");
}
