use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantEqualityComparisonBlock;

const FLAGGED_METHODS: &[&[u8]] = &[b"all?", b"any?", b"one?", b"none?"];

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
        if !FLAGGED_METHODS.iter().any(|m| *m == method_name) {
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

        // Must have exactly 1 block parameter
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

        let param = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };

        let param_name = param.name().as_slice();

        // Body should be a single equality comparison: x == value or value == x
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

        let eq_call = match stmts[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        let eq_method = eq_call.name().as_slice();
        let is_equality = eq_method == b"==";
        let is_regexp = eq_method == b"=~" || eq_method == b"match?";

        if !is_equality && !(is_regexp && !allow_regexp_match) {
            return;
        }

        // Check that one side of the comparison is the block parameter
        let recv = match eq_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let args = match eq_call.arguments() {
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
