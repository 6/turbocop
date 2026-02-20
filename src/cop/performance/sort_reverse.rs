use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct SortReverse;

impl Cop for SortReverse {
    fn name(&self) -> &'static str {
        "Performance/SortReverse"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        // This cop detects `sort { |a, b| b <=> a }` and suggests `.sort.reverse`.
        // Look for a `sort` call with a block that has exactly `b <=> a` (reversed
        // spaceship comparison).
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"sort" {
            return;
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        // Must have exactly 2 block parameters
        let block_params = match block.parameters() {
            Some(p) => match p.as_block_parameters_node() {
                Some(bp) => bp,
                None => return,
            },
            None => return,
        };
        let params_inner = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };
        let requireds: Vec<_> = params_inner.requireds().iter().collect();
        if requireds.len() != 2 {
            return;
        }
        let param_a = match requireds[0].as_required_parameter_node() {
            Some(p) => p.name().as_slice().to_vec(),
            None => return,
        };
        let param_b = match requireds[1].as_required_parameter_node() {
            Some(p) => p.name().as_slice().to_vec(),
            None => return,
        };

        // Block body must be a single `b <=> a` expression (reversed order)
        let body = match block.body() {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };
        let stmts_list: Vec<_> = stmts.body().iter().collect();
        if stmts_list.len() != 1 {
            return;
        }
        let cmp_call = match stmts_list[0].as_call_node() {
            Some(c) => c,
            None => return,
        };
        if cmp_call.name().as_slice() != b"<=>" {
            return;
        }
        // The receiver should be param_b, argument should be param_a (reversed)
        let receiver = match cmp_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let recv_name = if let Some(lv) = receiver.as_local_variable_read_node() {
            lv.name().as_slice().to_vec()
        } else {
            return;
        };
        let args = match cmp_call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }
        let arg_name = if let Some(lv) = arg_list[0].as_local_variable_read_node() {
            lv.name().as_slice().to_vec()
        } else {
            return;
        };

        // Check reversed order: receiver=b, arg=a
        if recv_name == param_b && arg_name == param_a {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(source, line, column, "Use `sort.reverse` instead of `sort { |a, b| b <=> a }`.".to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SortReverse, "cops/performance/sort_reverse");
}
