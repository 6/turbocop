use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, INTEGER_NODE, STATEMENTS_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ChangeByZero;

/// Detects `change { ... }.by(0)` or `change(X, :y).by(0)`.
impl Cop for ChangeByZero {
    fn name(&self) -> &'static str {
        "RSpec/ChangeByZero"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, INTEGER_NODE, STATEMENTS_NODE]
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
        // Config: NegatedMatcher — name of a custom negated matcher (e.g. "not_change")
        let negated_matcher = config.get_str("NegatedMatcher", "");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for `not_to change { }` when NegatedMatcher is set
        if !negated_matcher.is_empty() {
            if method_name == b"not_to" || method_name == b"to_not" {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if !arg_list.is_empty() {
                        if let Some(inner) = arg_list[0].as_call_node() {
                            let inner_name = inner.name().as_slice();
                            if (inner_name == b"change"
                                || inner_name == b"a_block_changing"
                                || inner_name == b"changing")
                                && inner.receiver().is_none()
                            {
                                let loc = inner.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!("Prefer `{negated_matcher}` over `not_to change`."),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Look for `.by(0)` call
        if method_name != b"by" {
            return;
        }

        // Must have argument of 0
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let is_zero = if let Some(int_node) = arg_list[0].as_integer_node() {
            // Check the value is 0
            let loc = int_node.location();
            let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            text == b"0"
        } else {
            false
        };

        if !is_zero {
            return;
        }

        // Receiver must be change/a_block_changing/changing
        let change_call = match call.receiver() {
            Some(recv) => match recv.as_call_node() {
                Some(c) => c,
                None => return,
            },
            None => return,
        };

        let change_name = change_call.name().as_slice();
        if change_name != b"change"
            && change_name != b"a_block_changing"
            && change_name != b"changing"
        {
            return;
        }

        // No receiver on the change call (or it could be chained from expect)
        if change_call.receiver().is_some() {
            return;
        }

        // RuboCop's expect_change_with_block pattern requires the block body
        // to be a simple send with no arguments: (send (...) _).
        // If the change call has a block, validate the block body structure.
        if let Some(block) = change_call.block() {
            if let Some(bn) = block.as_block_node() {
                let body_ok = if let Some(body) = bn.body() {
                    // Check if body is a simple send with no arguments
                    if let Some(body_call) = body.as_call_node() {
                        body_call.receiver().is_some() && body_call.arguments().is_none()
                    } else if let Some(stmts) = body.as_statements_node() {
                        // StatementsNode with a single statement that is a simple send
                        let stmts_list: Vec<_> = stmts.body().iter().collect();
                        if stmts_list.len() == 1 {
                            if let Some(body_call) = stmts_list[0].as_call_node() {
                                body_call.receiver().is_some() && body_call.arguments().is_none()
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !body_ok {
                    return;
                }
            }
        }

        // Flag from the change call to the end of .by(0)
        let change_loc = change_call.location();
        let (line, column) = source.offset_to_line_col(change_loc.start_offset());
        let msg = if negated_matcher.is_empty() {
            "Prefer `not_to change` over `to change.by(0)`.".to_string()
        } else {
            format!("Prefer `{negated_matcher}` over `to change.by(0)`.")
        };
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ChangeByZero, "cops/rspec/change_by_zero");

    #[test]
    fn negated_matcher_flags_not_to_change() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "NegatedMatcher".into(),
                serde_yml::Value::String("not_change".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"expect { x }.not_to change { y }\n";
        let diags = crate::testutil::run_cop_full_with_config(&ChangeByZero, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("not_change"));
    }

    #[test]
    fn negated_matcher_empty_does_not_flag_not_to_change() {
        let source = b"expect { x }.not_to change { y }\n";
        let diags = crate::testutil::run_cop_full(&ChangeByZero, source);
        assert!(diags.is_empty());
    }
}
