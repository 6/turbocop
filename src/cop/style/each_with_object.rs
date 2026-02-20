use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use crate::cop::node_type::{ARRAY_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, FALSE_NODE, FLOAT_NODE, HASH_NODE, IMAGINARY_NODE, INTEGER_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE, NIL_NODE, RATIONAL_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE};

pub struct EachWithObject;

/// Check if the accumulator variable is reassigned anywhere in the block body.
/// This covers `acc = ...`, `acc += ...`, `acc ||= ...`, etc.
fn accumulator_reassigned_in_body(node: &ruby_prism::Node<'_>, acc_name: &[u8]) -> bool {
    let mut finder = AccReassignFinder {
        acc_name: acc_name.to_vec(),
        found: false,
    };
    finder.visit(node);
    finder.found
}

struct AccReassignFinder {
    acc_name: Vec<u8>,
    found: bool,
}

impl<'pr> Visit<'pr> for AccReassignFinder {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.acc_name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_operator_write_node(&mut self, node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>) {
        if node.name().as_slice() == self.acc_name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    fn visit_local_variable_and_write_node(&mut self, node: &ruby_prism::LocalVariableAndWriteNode<'pr>) {
        if node.name().as_slice() == self.acc_name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    fn visit_local_variable_or_write_node(&mut self, node: &ruby_prism::LocalVariableOrWriteNode<'pr>) {
        if node.name().as_slice() == self.acc_name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }
}

impl Cop for EachWithObject {
    fn name(&self) -> &'static str {
        "Style/EachWithObject"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, FALSE_NODE, FLOAT_NODE, HASH_NODE, IMAGINARY_NODE, INTEGER_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE, NIL_NODE, RATIONAL_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE]
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

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name != "inject" && method_name != "reduce" {
            return;
        }

        // Must have arguments (the initial value)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // Initial value must be a hash or array literal (mutable collection)
        let initial = &arg_list[0];
        let is_mutable = initial.as_hash_node().is_some()
            || initial.as_keyword_hash_node().is_some()
            || initial.as_array_node().is_some();
        if !is_mutable {
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

        // Block must have at least 2 parameters
        if let Some(params) = block_node.parameters() {
            if let Some(block_params) = params.as_block_parameters_node() {
                if let Some(inner_params) = block_params.parameters() {
                    let requireds: Vec<_> = inner_params.requireds().iter().collect();
                    if requireds.len() < 2 {
                        return;
                    }
                } else {
                    return;
                }
            } else {
                return;
            }
        } else {
            // No parameters at all - skip
            return;
        }

        // The initial value must not be a basic literal (integer, float, string, symbol).
        // RuboCop's `simple_method_arg?` checks `method_arg&.basic_literal?`.
        let is_basic_literal = initial.as_integer_node().is_some()
            || initial.as_float_node().is_some()
            || initial.as_string_node().is_some()
            || initial.as_symbol_node().is_some()
            || initial.as_rational_node().is_some()
            || initial.as_imaginary_node().is_some()
            || initial.as_nil_node().is_some()
            || initial.as_true_node().is_some()
            || initial.as_false_node().is_some();
        if is_basic_literal {
            return;
        }

        // Check that the block body's last expression returns the accumulator variable.
        // In inject/reduce, the accumulator is the FIRST block parameter: |acc, elem|
        let acc_name = {
            let params = block_node.parameters().unwrap();
            let bp = params.as_block_parameters_node().unwrap();
            let inner = bp.parameters().unwrap();
            let requireds: Vec<_> = inner.requireds().iter().collect();
            if requireds.len() < 2 {
                return;
            }
            if let Some(rp) = requireds[0].as_required_parameter_node() {
                rp.name().as_slice().to_vec()
            } else {
                return;
            }
        };

        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_stmts: Vec<_> = stmts.body().iter().collect();
        if body_stmts.is_empty() {
            return;
        }

        // Last expression must be a local variable read matching the accumulator
        let last = &body_stmts[body_stmts.len() - 1];
        if let Some(lv) = last.as_local_variable_read_node() {
            if lv.name().as_slice() != acc_name {
                return;
            }
        } else {
            return;
        }

        // If the accumulator variable is assigned to within the block body,
        // we can't safely convert to each_with_object. With each_with_object,
        // the object is passed by reference and reassignment (`acc = ...` or
        // `acc += ...`) wouldn't propagate back to the caller.
        if accumulator_reassigned_in_body(&body, &acc_name) {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `each_with_object` instead of `{}`.", method_name),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EachWithObject, "cops/style/each_with_object");
}
