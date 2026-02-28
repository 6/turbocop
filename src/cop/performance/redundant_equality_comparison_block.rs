use ruby_prism::Visit;

use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantEqualityComparisonBlock;

const FLAGGED_METHODS: &[&[u8]] = &[b"all?", b"any?", b"one?", b"none?"];

/// Visitor that checks if any descendant node is a local variable read
/// matching the given name.
struct LvarFinder<'a> {
    name: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for LvarFinder<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
    }
}

/// Check if a node or any of its descendants references a local variable with the given name.
fn node_references_lvar(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    if let Some(lvar) = node.as_local_variable_read_node() {
        if lvar.name().as_slice() == name {
            return true;
        }
    }
    let mut finder = LvarFinder { name, found: false };
    finder.visit(node);
    finder.found
}

impl Cop for RedundantEqualityComparisonBlock {
    fn name(&self) -> &'static str {
        "Performance/RedundantEqualityComparisonBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
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
        let allow_regexp_match = config.get_bool("AllowRegexpMatch", true);
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if !FLAGGED_METHODS.contains(&method_name) {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Must have exactly 1 block parameter (no destructuring)
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
        if requireds.len() != 1 {
            return;
        }

        // Skip trailing comma destructuring: |type,| — Prism represents this as
        // a rest parameter (ImplicitRestNode)
        if param_list.rest().is_some() {
            return;
        }

        let param = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };

        let param_name = param.name().as_slice();

        // Body should be a single statement
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let statements = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let stmts: Vec<_> = statements.body().iter().collect();
        if stmts.len() != 1 {
            return;
        }

        let body_call = match stmts[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        let body_method = body_call.name().as_slice();

        // Check for is_a?/kind_of? pattern: item.is_a?(String)
        if body_method == b"is_a?" || body_method == b"kind_of?" {
            if self.check_is_a_pattern(&body_call, param_name) {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `grep` or `===` comparison instead of block with `==`.".to_string(),
                ));
            }
            return;
        }

        let is_equality = body_method == b"==";
        let is_case_equality = body_method == b"===";
        let is_regexp = body_method == b"=~" || body_method == b"match?";

        if !(is_equality || is_case_equality || (is_regexp && !allow_regexp_match)) {
            return;
        }

        // Check that one side of the comparison is the block parameter
        let recv = match body_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let args = match body_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_nodes: Vec<_> = args.arguments().iter().collect();
        if arg_nodes.len() != 1 {
            return;
        }

        let recv_is_param = recv
            .as_local_variable_read_node()
            .is_some_and(|lv| lv.name().as_slice() == param_name);

        let arg_is_param = arg_nodes[0]
            .as_local_variable_read_node()
            .is_some_and(|lv| lv.name().as_slice() == param_name);

        if !recv_is_param && !arg_is_param {
            return;
        }

        // For ===, only flag when the block param is on the RHS (the argument).
        // `Pattern === m` is flagged (m is arg), `m === pattern` is NOT (m is receiver).
        if is_case_equality && !arg_is_param {
            return;
        }

        // For == (and =~/match?), if param appears on both sides, skip —
        // the comparison can't be simplified.
        if recv_is_param && arg_is_param {
            return;
        }

        // Check if the param is used in the OTHER side of the comparison.
        // e.g., `arr.any? { |bin| num[0, bin.size] == bin }` — bin appears
        // in both the receiver subtree and as the argument.
        if recv_is_param && node_references_lvar(&arg_nodes[0], param_name) {
            return;
        }
        if arg_is_param && node_references_lvar(&recv, param_name) {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let msg = if is_regexp {
            "Use `grep` instead of block with regexp comparison."
        } else {
            "Use `grep` or `===` comparison instead of block with `==`."
        };
        diagnostics.push(self.diagnostic(source, line, column, msg.to_string()));
    }
}

impl RedundantEqualityComparisonBlock {
    /// Check if a `is_a?`/`kind_of?` call pattern is an offense.
    /// Pattern: `item.is_a?(String)` where `item` is the block param.
    /// The block param must be the RECEIVER, not the argument.
    fn check_is_a_pattern(&self, call: &ruby_prism::CallNode<'_>, param_name: &[u8]) -> bool {
        // Receiver must be the block param
        let recv = match call.receiver() {
            Some(r) => r,
            None => return false,
        };

        let recv_is_param = recv
            .as_local_variable_read_node()
            .is_some_and(|lv| lv.name().as_slice() == param_name);

        if !recv_is_param {
            return false;
        }

        // Must have exactly 1 argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return false,
        };

        let arg_nodes: Vec<_> = args.arguments().iter().collect();
        if arg_nodes.len() != 1 {
            return false;
        }

        // The argument must NOT reference the block param.
        // e.g., `klasses.all? { |klass| item.is_a?(klass) }` is NOT an offense
        // because the block param is the argument, not a constant class name.
        if node_references_lvar(&arg_nodes[0], param_name) {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantEqualityComparisonBlock,
        "cops/performance/redundant_equality_comparison_block"
    );

    #[test]
    fn config_allow_regexp_match_false_flags_match() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("AllowRegexpMatch".into(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let source = b"items.all? { |item| item =~ /pattern/ }\n";
        let diags = run_cop_full_with_config(&RedundantEqualityComparisonBlock, source, config);
        assert!(
            !diags.is_empty(),
            "Should flag =~ when AllowRegexpMatch:false"
        );
    }

    #[test]
    fn config_allow_regexp_match_true_allows_match() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("AllowRegexpMatch".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = b"items.all? { |item| item =~ /pattern/ }\n";
        let diags = run_cop_full_with_config(&RedundantEqualityComparisonBlock, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag =~ when AllowRegexpMatch:true"
        );
    }
}
