use crate::cop::node_type::{
    BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct CompactBlank;

/// Destructive methods map to `compact_blank!`, non-destructive to `compact_blank`.
fn is_destructive(method_name: &[u8]) -> bool {
    method_name == b"delete_if" || method_name == b"keep_if"
}

fn preferred_method(method_name: &[u8]) -> &'static str {
    if is_destructive(method_name) {
        "compact_blank!"
    } else {
        "compact_blank"
    }
}

impl Cop for CompactBlank {
    fn name(&self) -> &'static str {
        "Rails/CompactBlank"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_ARGUMENT_NODE,
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
            SYMBOL_NODE,
        ]
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
        // minimum_target_rails_version 6.1
        if !config.rails_version_at_least(6.1) {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Expected predicate for each method family:
        // reject/delete_if → blank?
        // select/filter/keep_if → present?
        let expected_predicate: &[u8] = match method_name {
            b"reject" | b"delete_if" => b"blank?",
            b"select" | b"filter" | b"keep_if" => b"present?",
            _ => return,
        };

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        // Must have a block (either block-pass &:blank? or { |e| e.blank? })
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        // Check for block-pass form: reject(&:blank?), select(&:present?)
        if let Some(block_arg) = block.as_block_argument_node() {
            if let Some(expr) = block_arg.expression() {
                if let Some(sym) = expr.as_symbol_node() {
                    let unescaped = sym.unescaped();
                    if unescaped == expected_predicate {
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Use `{}` instead.", preferred_method(method_name)),
                        ));
                    }
                }
            }
            return;
        }

        // Check for block form: reject { |e| e.blank? } or reject { |_k, v| v.blank? }
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let param_list = match block_params.parameters() {
            Some(pl) => pl,
            None => return,
        };
        let requireds: Vec<_> = param_list.requireds().iter().collect();

        // Single arg: reject { |e| e.blank? }
        // Two args (hash form): reject { |_k, v| v.blank? }
        let check_param_name = match requireds.len() {
            1 => match requireds[0].as_required_parameter_node() {
                Some(p) => p.name().as_slice().to_vec(),
                None => return,
            },
            2 => {
                // Hash form: the SECOND argument is the value
                match requireds[1].as_required_parameter_node() {
                    Some(p) => p.name().as_slice().to_vec(),
                    None => return,
                }
            }
            _ => return,
        };

        // Block body should be a single call to .blank? or .present?
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<ruby_prism::Node<'_>> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        let body_call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        if body_call.name().as_slice() != expected_predicate {
            return;
        }

        // The receiver of .blank?/.present? must be the relevant block parameter
        let recv = match body_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let recv_lvar = match recv.as_local_variable_read_node() {
            Some(lv) => lv,
            None => return,
        };
        if recv_lvar.name().as_slice() != check_param_name.as_slice() {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}` instead.", preferred_method(method_name)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use std::collections::HashMap;

    fn config_with_rails(version: f64) -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "TargetRailsVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(version)),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn offense_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &CompactBlank,
            include_bytes!("../../../tests/fixtures/cops/rails/compact_blank/offense.rb"),
            config_with_rails(6.1),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &CompactBlank,
            include_bytes!("../../../tests/fixtures/cops/rails/compact_blank/no_offense.rb"),
            config_with_rails(6.1),
        );
    }

    #[test]
    fn skipped_when_rails_below_6_1() {
        let source = b"collection.reject(&:blank?)\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &CompactBlank,
            source,
            config_with_rails(6.0),
            "test.rb",
        );
        assert!(diagnostics.is_empty(), "Should not fire on Rails < 6.1");
    }
}
