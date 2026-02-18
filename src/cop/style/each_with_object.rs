use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EachWithObject;

impl Cop for EachWithObject {
    fn name(&self) -> &'static str {
        "Style/EachWithObject"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name != "inject" && method_name != "reduce" {
            return Vec::new();
        }

        // Must have arguments (the initial value)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        // Initial value must be a hash or array literal (mutable collection)
        let initial = &arg_list[0];
        let is_mutable = initial.as_hash_node().is_some()
            || initial.as_keyword_hash_node().is_some()
            || initial.as_array_node().is_some();
        if !is_mutable {
            return Vec::new();
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Block must have at least 2 parameters
        if let Some(params) = block_node.parameters() {
            if let Some(block_params) = params.as_block_parameters_node() {
                if let Some(inner_params) = block_params.parameters() {
                    let requireds: Vec<_> = inner_params.requireds().iter().collect();
                    if requireds.len() < 2 {
                        return Vec::new();
                    }
                } else {
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }
        } else {
            // No parameters at all - skip
            return Vec::new();
        }

        // Check that the block body's last expression returns the accumulator variable.
        // In inject/reduce, the accumulator is the FIRST block parameter: |acc, elem|
        let acc_name = {
            let params = block_node.parameters().unwrap();
            let bp = params.as_block_parameters_node().unwrap();
            let inner = bp.parameters().unwrap();
            let requireds: Vec<_> = inner.requireds().iter().collect();
            if requireds.len() < 2 {
                return Vec::new();
            }
            if let Some(rp) = requireds[0].as_required_parameter_node() {
                rp.name().as_slice().to_vec()
            } else {
                return Vec::new();
            }
        };

        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_stmts: Vec<_> = stmts.body().iter().collect();
        if body_stmts.is_empty() {
            return Vec::new();
        }

        // Last expression must be a local variable read matching the accumulator
        let last = &body_stmts[body_stmts.len() - 1];
        if let Some(lv) = last.as_local_variable_read_node() {
            if lv.name().as_slice() != acc_name {
                return Vec::new();
            }
        } else {
            return Vec::new();
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `each_with_object` instead of `{}`.", method_name),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EachWithObject, "cops/style/each_with_object");
}
