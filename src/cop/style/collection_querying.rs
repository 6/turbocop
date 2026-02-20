use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE, INTEGER_NODE};

pub struct CollectionQuerying;

impl Cop for CollectionQuerying {
    fn name(&self) -> &'static str {
        "Style/CollectionQuerying"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE, INTEGER_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        let active_support = config.get_bool("ActiveSupportExtensionsEnabled", false);

        // Pattern: x.count.positive? => x.any?
        // Pattern: x.count.zero? => x.none?
        if method_name == "positive?" || method_name == "zero?" {
            if call.arguments().is_some() {
                return;
            }

            if let Some(receiver) = call.receiver() {
                if let Some(recv_call) = receiver.as_call_node() {
                    let recv_method =
                        std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
                    // Only check `count`, NOT `length` or `size` — those yield false
                    // positives because String and other non-Enumerable classes implement them.
                    if recv_method == "count" && recv_call.receiver().is_some() {
                        // count must not have positional arguments (block is OK)
                        if !has_positional_args(&recv_call) {
                            let suggestion = if method_name == "positive?" {
                                "any?"
                            } else {
                                "none?"
                            };

                            let loc =
                                recv_call.message_loc().unwrap_or(recv_call.location());
                            let (line, column) =
                                source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Use `{}` instead.", suggestion),
                            ));
                        }
                    }
                }
            }
        }

        // Pattern: x.count > 0 => x.any?
        // Pattern: x.count == 0 => x.none?
        // Pattern: x.count != 0 => x.any?
        // Pattern: x.count == 1 => x.one?
        // Pattern: x.count > 1 => x.many? (only with ActiveSupportExtensionsEnabled)
        if matches!(method_name, ">" | "==" | "!=") {
            if let Some(receiver) = call.receiver() {
                if let Some(recv_call) = receiver.as_call_node() {
                    let recv_method =
                        std::str::from_utf8(recv_call.name().as_slice()).unwrap_or("");
                    if recv_method == "count"
                        && recv_call.receiver().is_some()
                        && !has_positional_args(&recv_call)
                    {
                        if let Some(args) = call.arguments() {
                            let arg_list: Vec<_> = args.arguments().iter().collect();
                            if arg_list.len() == 1 {
                                if let Some(int_node) = arg_list[0].as_integer_node() {
                                    let src = std::str::from_utf8(
                                        int_node.location().as_slice(),
                                    )
                                    .unwrap_or("");
                                    if let Ok(v) = src.parse::<i64>() {
                                        let suggestion = match (method_name, v) {
                                            (">" | "!=", 0) => Some("any?"),
                                            ("==", 0) => Some("none?"),
                                            ("==", 1) => Some("one?"),
                                            (">", 1) if active_support => {
                                                Some("many?")
                                            }
                                            _ => None,
                                        };
                                        if let Some(suggestion) = suggestion {
                                            let loc = recv_call
                                                .message_loc()
                                                .unwrap_or(recv_call.location());
                                            let (line, column) = source
                                                .offset_to_line_col(loc.start_offset());
                                            diagnostics.push(self.diagnostic(
                                                source,
                                                line,
                                                column,
                                                format!(
                                                    "Use `{}` instead.",
                                                    suggestion
                                                ),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

    }
}

/// Check if a call node has positional arguments (not just a block-pass or no args).
/// RuboCop only flags `count` without arguments or with a block (count(&:foo?) or count { ... }).
fn has_positional_args(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            // BlockArgumentNode is `&:foo?` — this is OK
            if arg.as_block_argument_node().is_none() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CollectionQuerying, "cops/style/collection_querying");
}
