use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, ASSOC_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, FALSE_NODE, FLOAT_NODE, HASH_NODE, INTEGER_NODE, INTERPOLATED_STRING_NODE, NIL_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE};

pub struct ReturnFromStub;

/// Default style is `and_return` — flags block-style stubs returning static values.
/// Detects: `allow(X).to receive(:y) { static_value }`
impl Cop for ReturnFromStub {
    fn name(&self) -> &'static str {
        "RSpec/ReturnFromStub"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, ASSOC_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, FALSE_NODE, FLOAT_NODE, HASH_NODE, INTEGER_NODE, INTERPOLATED_STRING_NODE, NIL_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE]
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
        // Config: EnforcedStyle — "and_return" (default) or "block"
        let enforced_style = config.get_str("EnforcedStyle", "and_return");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // "block" style: flag `.and_return(value)` — prefer block form
        if enforced_style == "block" {
            if method_name == b"and_return" {
                if let Some(recv) = call.receiver() {
                    if recv.as_call_node().is_some() {
                        if let Some(args) = call.arguments() {
                            let arg_list: Vec<_> = args.arguments().iter().collect();
                            if !arg_list.is_empty() && arg_list.iter().all(|a| is_static_value(a)) {
                                let loc = call.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Use a block for static values.".to_string(),
                                ));
                            }
                        }
                    }
                }
            }
            return;
        }

        // Default "and_return" style: flag block-style stubs returning static values
        // We need `.to` or `.not_to`
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        // Check receiver is allow/expect
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };
        let recv_name = recv_call.name().as_slice();
        if recv_name != b"allow" && recv_name != b"expect" {
            return;
        }
        if recv_call.receiver().is_some() {
            return;
        }

        // Get the argument chain (receive(:y) or receive(:y).with(...))
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Find the `receive` call in the argument chain and check for a block on it
        let block_on_receive = find_block_on_receive_chain(&arg_list[0]);
        // Also check for a block on `.to` itself
        let block_on_to = call.block();

        let block_node = if let Some(b) = block_on_receive {
            b
        } else if let Some(b) = block_on_to {
            match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            }
        } else {
            return;
        };

        // If block has parameters, it's a dynamic block
        if let Some(params) = block_node.parameters() {
            if let Some(bp) = params.as_block_parameters_node() {
                if let Some(p) = bp.parameters() {
                    let req: Vec<_> = p.requireds().iter().collect();
                    if !req.is_empty() {
                        return;
                    }
                }
            }
        }

        let body = match block_node.body() {
            Some(b) => b,
            None => {
                let loc = block_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `and_return` for static values.".to_string(),
                ));
                return;
            }
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let stmt_list: Vec<_> = stmts.body().iter().collect();
        if stmt_list.is_empty() {
            return;
        }

        let all_static = stmt_list.iter().all(|s| is_static_value(s));
        if !all_static {
            return;
        }

        let loc = block_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `and_return` for static values.".to_string(),
        ));
    }
}

fn find_block_on_receive_chain<'a>(
    node: &ruby_prism::Node<'a>,
) -> Option<ruby_prism::BlockNode<'a>> {
    let call = node.as_call_node()?;
    let mut current = call;
    loop {
        if current.block().is_some() {
            if let Some(block) = current.block() {
                return block.as_block_node();
            }
        }
        let recv = current.receiver()?;
        current = recv.as_call_node()?;
    }
}

fn is_static_value(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
    {
        return true;
    }

    if node.as_interpolated_string_node().is_some() {
        return false;
    }

    if let Some(arr) = node.as_array_node() {
        return arr.elements().iter().all(|e| is_static_value(&e));
    }

    // Note: keyword_hash_node (keyword args) intentionally not handled —
    // only hash literals can appear as static return values in stubs.
    if let Some(hash) = node.as_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_static_value(&assoc.key()) && is_static_value(&assoc.value())
            } else {
                false
            }
        });
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReturnFromStub, "cops/rspec/return_from_stub");

    #[test]
    fn block_style_flags_and_return() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("block".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"allow(foo).to receive(:bar).and_return(42)\n";
        let diags = crate::testutil::run_cop_full_with_config(&ReturnFromStub, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("block"));
    }

    #[test]
    fn block_style_does_not_flag_block_form() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("block".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"allow(foo).to receive(:bar) { 42 }\n";
        let diags = crate::testutil::run_cop_full_with_config(&ReturnFromStub, source, config);
        assert!(diags.is_empty());
    }
}
