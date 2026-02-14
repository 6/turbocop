use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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

        // Must be `around` (receiverless or on config)
        if call.name().as_slice() != b"around" {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Get the block parameter name
        let param_name = get_block_param_name(&block_node);

        match param_name {
            None => {
                // No block parameter — flag the whole around call
                // (unless the body uses _1.run or _1.call)
                if body_uses_numbered_param_run(&block_node) {
                    return Vec::new();
                }
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Test object should be passed to around block.".to_string(),
                )]
            }
            Some(name) => {
                // Has a block parameter — check if it's used with .run or .call
                if body_uses_param_correctly(&block_node, &name) {
                    return Vec::new();
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
                                return vec![self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!(
                                        "You should call `{name_str}.call` or `{name_str}.run`."
                                    ),
                                )];
                            }
                        }
                    }
                }

                Vec::new()
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
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };

    for stmt in stmts.body().iter() {
        if node_uses_param_correctly(&stmt, param_name) {
            return true;
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

        // Recurse into block body
        if let Some(block) = call.block() {
            if let Some(bn) = block.as_block_node() {
                if let Some(body) = bn.body() {
                    if let Some(stmts) = body.as_statements_node() {
                        for s in stmts.body().iter() {
                            if node_uses_param_correctly(&s, param_name) {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // Recurse into receiver
        if let Some(recv) = call.receiver() {
            if node_uses_param_correctly(&recv, param_name) {
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
