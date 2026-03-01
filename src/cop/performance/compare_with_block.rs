use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct CompareWithBlock;

impl Cop for CompareWithBlock {
    fn name(&self) -> &'static str {
        "Performance/CompareWithBlock"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        let replacement = match method_name {
            b"sort" => "sort_by",
            b"sort!" => "sort_by!",
            b"min" => "min_by",
            b"max" => "max_by",
            b"minmax" => "minmax_by",
            _ => return,
        };
        let method_str = std::str::from_utf8(method_name).unwrap_or("sort");

        if call.receiver().is_none() {
            return;
        }

        // Skip safe navigation (&.sort) — RuboCop only matches `send`, not `csend`
        if call.call_operator_loc().is_some() {
            let op = call.call_operator_loc().unwrap();
            let op_bytes = &source.as_bytes()[op.start_offset()..op.end_offset()];
            if op_bytes == b"&." {
                return;
            }
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Must have exactly 2 block parameters
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
        if requireds.len() != 2 {
            return;
        }

        let param_a = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };
        let param_b = match requireds[1].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };

        let name_a = param_a.name().as_slice();
        let name_b = param_b.name().as_slice();

        // Body should be a single `a.method <=> b.method` call
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

        let spaceship_call = match stmts[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        if spaceship_call.name().as_slice() != b"<=>" {
            return;
        }

        // Check receiver is a method call on param_a: a.method
        let recv = match spaceship_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let recv_receiver = match recv_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_var = match recv_receiver.as_local_variable_read_node() {
            Some(lv) => lv,
            None => return,
        };

        if recv_var.name().as_slice() != name_a {
            return;
        }

        // Check argument is a method call on param_b: b.method
        let args = match spaceship_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_nodes: Vec<_> = args.arguments().iter().collect();
        if arg_nodes.len() != 1 {
            return;
        }

        let arg_call = match arg_nodes[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        let arg_receiver = match arg_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let arg_var = match arg_receiver.as_local_variable_read_node() {
            Some(lv) => lv,
            None => return,
        };

        if arg_var.name().as_slice() != name_b {
            return;
        }

        // Both should call the same method
        let method_a = recv_call.name().as_slice();
        let method_b = arg_call.name().as_slice();
        if method_a != method_b {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        if method_a == b"[]" {
            // For [] indexing: require exactly 1 argument that is a literal (sym/str/int),
            // and both sides must use the same key.
            let args_a: Vec<_> = recv_call
                .arguments()
                .map_or(vec![], |a| a.arguments().iter().collect());
            let args_b: Vec<_> = arg_call
                .arguments()
                .map_or(vec![], |a| a.arguments().iter().collect());
            if args_a.len() != 1 || args_b.len() != 1 {
                return;
            }
            let key_a = &args_a[0];
            let key_b = &args_b[0];
            // Must be a literal type (string, symbol, or integer)
            let is_literal = key_a.as_string_node().is_some()
                || key_a.as_symbol_node().is_some()
                || key_a.as_integer_node().is_some();
            if !is_literal {
                return;
            }
            // Both keys must be the same literal (compare source bytes)
            let key_a_src =
                &source.as_bytes()[key_a.location().start_offset()..key_a.location().end_offset()];
            let key_b_src =
                &source.as_bytes()[key_b.location().start_offset()..key_b.location().end_offset()];
            if key_a_src != key_b_src {
                return;
            }
            let key_display = std::str::from_utf8(key_a_src).unwrap_or("key");
            let var_a_str = std::str::from_utf8(name_a).unwrap_or("a");
            let var_b_str = std::str::from_utf8(name_b).unwrap_or("b");
            diagnostics.push(self.diagnostic(source, line, column,
                format!("Use `{replacement} {{ |a| a[{key_display}] }}` instead of `{method_str} {{ |{var_a_str}, {var_b_str}| {var_a_str}[{key_display}] <=> {var_b_str}[{key_display}] }}`.")));
        } else {
            // For regular method calls: require zero arguments
            if recv_call.arguments().is_some() || arg_call.arguments().is_some() {
                return;
            }
            let attr_method = std::str::from_utf8(method_a).unwrap_or("method");
            let var_a_str = std::str::from_utf8(name_a).unwrap_or("a");
            let var_b_str = std::str::from_utf8(name_b).unwrap_or("b");
            diagnostics.push(
                self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{replacement}(&:{attr_method})` instead of `{method_str} {{ |{var_a_str}, {var_b_str}| {var_a_str}.{attr_method} <=> {var_b_str}.{attr_method} }}`.")
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CompareWithBlock, "cops/performance/compare_with_block");

    #[test]
    fn detects_do_end_block() {
        use crate::testutil::run_cop_full;
        let source = b"arr.sort do |a, b|\n  a.name <=> b.name\nend\n";
        let diags = run_cop_full(&CompareWithBlock, source);
        assert!(!diags.is_empty(), "Should flag do..end block form");
    }

    #[test]
    fn detects_multiline_brace_block() {
        use crate::testutil::run_cop_full;
        let source = b"arr.min { |a, b|\n  a.name <=> b.name\n}\n";
        let diags = run_cop_full(&CompareWithBlock, source);
        assert!(!diags.is_empty(), "Should flag multiline brace block");
    }

    #[test]
    fn debug_various_patterns() {
        use crate::testutil::run_cop_full;
        // Pattern: receiver.method_call (no args) — the standard case
        let source = b"[3,1,2].sort { |a, b| a.to_s <=> b.to_s }\n";
        let diags = run_cop_full(&CompareWithBlock, source);
        assert!(!diags.is_empty(), "Should flag [3,1,2].sort with to_s");

        // Pattern: method with arguments like a.fetch(:key)
        let source = b"arr.sort { |a, b| a.fetch(:key) <=> b.fetch(:key) }\n";
        let diags = run_cop_full(&CompareWithBlock, source);
        assert!(diags.is_empty(), "Should NOT flag fetch with args");

        // Pattern: multiline do..end with min
        let source = b"list.min do |a, b|\n  a.size <=> b.size\nend\n";
        let diags = run_cop_full(&CompareWithBlock, source);
        assert!(!diags.is_empty(), "Should flag multiline do..end min");

        // Pattern: minmax with block
        let source = b"items.minmax { |a, b| a.length <=> b.length }\n";
        let diags = run_cop_full(&CompareWithBlock, source);
        assert!(!diags.is_empty(), "Should flag minmax");

        // Pattern: max with index bracket
        let source = b"data.max { |a, b| a[:value] <=> b[:value] }\n";
        let diags = run_cop_full(&CompareWithBlock, source);
        assert!(!diags.is_empty(), "Should flag max with index bracket");
    }
}
