use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BEGIN_NODE, BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, ELSE_NODE, IF_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE, NEXT_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, YIELD_NODE};

pub struct AroundBlock;

/// Flags `around` hooks that don't yield or call `run`/`call` on the example.
/// The test object should be executed within the around block.
impl Cop for AroundBlock {
    fn name(&self) -> &'static str {
        "RSpec/AroundBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, ELSE_NODE, IF_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE, NEXT_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, YIELD_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be `around` (receiverless or on config)
        if call.name().as_slice() != b"around" {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Get the block parameter name
        let param_name = get_block_param_name(&block_node);

        match param_name {
            None => {
                // No block parameter — flag the whole around call
                // (unless the body uses _1.run/_1.call or yield)
                if body_uses_numbered_param_run(&block_node) || body_contains_yield(&block_node) {
                    return;
                }
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Test object should be passed to around block.".to_string(),
                ));
            }
            Some(name) => {
                // Has a block parameter — check if it's used with .run or .call (or yield)
                if body_uses_param_correctly(&block_node, &name) || body_contains_yield(&block_node) {
                    return;
                }

                // Flag the parameter itself
                if let Some(params) = block_node.parameters() {
                    if let Some(bp) = params.as_block_parameters_node() {
                        if let Some(p) = bp.parameters() {
                            let requireds: Vec<_> = p.requireds().iter().collect();
                            if !requireds.is_empty() {
                                let param_loc = requireds[0].location();
                                let (line, column) =
                                    source.offset_to_line_col(param_loc.start_offset());
                                let name_str = std::str::from_utf8(&name).unwrap_or("example");
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!(
                                        "You should call `{name_str}.call` or `{name_str}.run`."
                                    ),
                                ));
                            }
                        }
                    }
                }


            }
        }
    }
}

fn get_block_param_name(block: &ruby_prism::BlockNode<'_>) -> Option<Vec<u8>> {
    let params = block.parameters()?;
    let bp = params.as_block_parameters_node()?;
    let p = bp.parameters()?;
    let requireds: Vec<_> = p.requireds().iter().collect();
    if requireds.is_empty() {
        return None;
    }
    // Get the name of the first required parameter
    if let Some(rp) = requireds[0].as_required_parameter_node() {
        Some(rp.name().as_slice().to_vec())
    } else {
        None
    }
}

fn body_uses_param_correctly(block: &ruby_prism::BlockNode<'_>, param_name: &[u8]) -> bool {
    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };
    node_tree_uses_param(&body, param_name)
}

/// Recursively search a node tree (including BeginNode, StatementsNode) for param usage.
fn node_tree_uses_param(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    if node_uses_param_correctly(node, param_name) {
        return true;
    }

    // Handle StatementsNode
    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            if node_tree_uses_param(&stmt, param_name) {
                return true;
            }
        }
    }

    // Handle BeginNode (for blocks with rescue/ensure)
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                if node_tree_uses_param(&s, param_name) {
                    return true;
                }
            }
        }
        // Check rescue clauses
        let mut rescue = begin_node.rescue_clause();
        while let Some(rc) = rescue {
            if let Some(stmts) = rc.statements() {
                for s in stmts.body().iter() {
                    if node_tree_uses_param(&s, param_name) {
                        return true;
                    }
                }
            }
            rescue = rc.subsequent();
        }
        // Check ensure clause
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure_clause.statements() {
                for s in stmts.body().iter() {
                    if node_tree_uses_param(&s, param_name) {
                        return true;
                    }
                }
            }
        }
    }

    // Handle local variable assignments (e.g., measurement = Benchmark.measure { example.run })
    if let Some(lv) = node.as_local_variable_write_node() {
        if node_tree_uses_param(&lv.value(), param_name) {
            return true;
        }
    }

    // Handle IfNode / UnlessNode — recurse into predicate, if-body, and else/elsif
    if let Some(if_node) = node.as_if_node() {
        if node_tree_uses_param(&if_node.predicate(), param_name) {
            return true;
        }
        if let Some(stmts) = if_node.statements() {
            for s in stmts.body().iter() {
                if node_tree_uses_param(&s, param_name) {
                    return true;
                }
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            if node_tree_uses_param(&subsequent, param_name) {
                return true;
            }
        }
    }

    // Handle ElseNode
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for s in stmts.body().iter() {
                if node_tree_uses_param(&s, param_name) {
                    return true;
                }
            }
        }
    }

    // Handle `next expr` — the expression inside next may use the param
    if let Some(next_node) = node.as_next_node() {
        if let Some(args) = next_node.arguments() {
            for arg in args.arguments().iter() {
                if node_tree_uses_param(&arg, param_name) {
                    return true;
                }
            }
        }
    }

    false
}

fn node_uses_param_correctly(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    // Check for param.run or param.call
    if let Some(call) = node.as_call_node() {
        let method = call.name().as_slice();
        if method == b"run" || method == b"call" {
            if let Some(recv) = call.receiver() {
                if is_param_ref(&recv, param_name) {
                    return true;
                }
            }
        }

        // Check for passing param as a block arg: `1.times(&test)`
        if let Some(block_arg) = call.block() {
            if let Some(ba) = block_arg.as_block_argument_node() {
                if let Some(expr) = ba.expression() {
                    if is_param_ref(&expr, param_name) {
                        return true;
                    }
                }
            }
        }

        // Check for passing param as a regular argument: `something(test, ...)`
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if is_param_ref(&arg, param_name) {
                    return true;
                }
            }
        }

        // Recurse into block body (handles StatementsNode, BeginNode, etc.)
        if let Some(block) = call.block() {
            if let Some(bn) = block.as_block_node() {
                if let Some(body) = bn.body() {
                    if node_tree_uses_param(&body, param_name) {
                        return true;
                    }
                }
            }
        }

        // Recurse into receiver
        if let Some(recv) = call.receiver() {
            if node_tree_uses_param(&recv, param_name) {
                return true;
            }
        }
    }

    // Check for `yield(something, test)` pattern
    if let Some(yield_node) = node.as_yield_node() {
        if let Some(args) = yield_node.arguments() {
            for arg in args.arguments().iter() {
                if is_param_ref(&arg, param_name) {
                    return true;
                }
            }
        }
    }

    false
}

fn body_contains_yield(block: &ruby_prism::BlockNode<'_>) -> bool {
    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };
    node_tree_contains_yield(&body)
}

/// Recursively search a node tree (including BeginNode, StatementsNode) for yield.
fn node_tree_contains_yield(node: &ruby_prism::Node<'_>) -> bool {
    if node_contains_yield(node) {
        return true;
    }

    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            if node_tree_contains_yield(&stmt) {
                return true;
            }
        }
    }

    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                if node_tree_contains_yield(&s) {
                    return true;
                }
            }
        }
        let mut rescue = begin_node.rescue_clause();
        while let Some(rc) = rescue {
            if let Some(stmts) = rc.statements() {
                for s in stmts.body().iter() {
                    if node_tree_contains_yield(&s) {
                        return true;
                    }
                }
            }
            rescue = rc.subsequent();
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure_clause.statements() {
                for s in stmts.body().iter() {
                    if node_tree_contains_yield(&s) {
                        return true;
                    }
                }
            }
        }
    }

    if let Some(lv) = node.as_local_variable_write_node() {
        if node_tree_contains_yield(&lv.value()) {
            return true;
        }
    }

    false
}

fn node_contains_yield(node: &ruby_prism::Node<'_>) -> bool {
    // Direct yield node
    if node.as_yield_node().is_some() {
        return true;
    }

    // Check inside call nodes (e.g., in method chain or block)
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if node_contains_yield(&recv) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if node_contains_yield(&arg) {
                    return true;
                }
            }
        }
        if let Some(block) = call.block() {
            if let Some(bn) = block.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            if node_contains_yield(&s) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    // Check inside begin/rescue/ensure
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for s in stmts.body().iter() {
                if node_contains_yield(&s) {
                    return true;
                }
            }
        }
        if let Some(rescue) = begin_node.rescue_clause() {
            if let Some(stmts) = rescue.statements() {
                for s in stmts.body().iter() {
                    if node_contains_yield(&s) {
                        return true;
                    }
                }
            }
        }
        if let Some(ensure) = begin_node.ensure_clause() {
            if let Some(stmts) = ensure.statements() {
                for s in stmts.body().iter() {
                    if node_contains_yield(&s) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

fn body_uses_numbered_param_run(block: &ruby_prism::BlockNode<'_>) -> bool {
    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };

    for stmt in stmts.body().iter() {
        if node_uses_numbered_param_run(&stmt) {
            return true;
        }
    }
    false
}

fn node_uses_numbered_param_run(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let method = call.name().as_slice();
        if method == b"run" || method == b"call" {
            if let Some(recv) = call.receiver() {
                if let Some(rc) = recv.as_call_node() {
                    if rc.name().as_slice() == b"_1" && rc.receiver().is_none() {
                        return true;
                    }
                }
                if let Some(lv) = recv.as_local_variable_read_node() {
                    if lv.name().as_slice() == b"_1" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn is_param_ref(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    if let Some(lv) = node.as_local_variable_read_node() {
        return lv.name().as_slice() == param_name;
    }
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none()
            && call.arguments().is_none()
            && call.name().as_slice() == param_name
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AroundBlock, "cops/rspec/around_block");
}
