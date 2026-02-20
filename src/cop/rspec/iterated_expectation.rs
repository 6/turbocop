use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE};

pub struct IteratedExpectation;

impl Cop for IteratedExpectation {
    fn name(&self) -> &'static str {
        "RSpec/IteratedExpectation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE]
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
        // Flag `.each { |x| expect(x)... }` — suggest using `all` matcher
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"each" {
            return;
        }

        // Must have a receiver (the array/collection)
        if call.receiver().is_none() {
            return;
        }

        // Must have a block with a parameter
        let block_raw = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block = match block_raw.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Must have block parameters
        let params = match block.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_params = match params.as_block_parameters_node() {
            Some(p) => p,
            None => return,
        };

        let inner_params = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };

        let requireds: Vec<_> = inner_params.requireds().iter().collect();
        // RuboCop pattern requires exactly one block parameter: (args (arg $_))
        if requireds.len() != 1 {
            return;
        }

        // Check if the parameter starts with _ (unused)
        if let Some(first_param) = requireds.first() {
            if let Some(req) = first_param.as_required_parameter_node() {
                if req.name().as_slice().starts_with(b"_") {
                    return;
                }
            }
        }

        // Check if the block body is expect(block_param).to ...
        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        // Get block parameter name
        let param_name = if let Some(first_param) = requireds.first() {
            if let Some(req) = first_param.as_required_parameter_node() {
                req.name().as_slice().to_vec()
            } else {
                return;
            }
        } else {
            return;
        };

        // RuboCop requires ALL statements in the block body to be
        // expect(block_param).to ... where block_param is a bare lvar.
        if is_single_expectation_with_param(&body, &param_name)
            || is_all_expectations_with_param(&body, &param_name)
        {
            let recv = call.receiver().unwrap();
            let loc = recv.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer using the `all` matcher instead of iterating over an array.".to_string(),
            ));
        }

    }
}

/// Check if a node is `expect(param).to ...` where param is a bare local variable.
fn is_expectation_with_param(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    // RuboCop's pattern only matches `.to`, NOT `.not_to` or `.to_not`:
    //   (send (send nil? :expect (lvar %)) :to ...)
    let method = call.name().as_slice();
    if method != b"to" {
        return false;
    }

    // The receiver should be `expect(param)`
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };

    let expect_call = match recv.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if expect_call.receiver().is_some() || expect_call.name().as_slice() != b"expect" {
        return false;
    }

    // The argument to expect should be a bare local variable matching the block param
    let args = match expect_call.arguments() {
        Some(a) => a,
        None => return false,
    };

    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() != 1 {
        return false;
    }

    if let Some(lvar) = arg_list[0].as_local_variable_read_node() {
        lvar.name().as_slice() == param_name
    } else {
        false
    }
}

/// Check if a single node is an expectation with the param.
fn is_single_expectation_with_param(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    is_expectation_with_param(node, param_name)
}

/// Check if all statements in a begin/statements node are expectations with the param.
fn is_all_expectations_with_param(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    if let Some(stmts) = node.as_statements_node() {
        let children: Vec<_> = stmts.body().iter().collect();
        if children.is_empty() {
            return false;
        }
        children
            .iter()
            .all(|child| is_expectation_with_param(child, param_name))
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IteratedExpectation, "cops/rspec/iterated_expectation");
}
