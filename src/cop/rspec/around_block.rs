use crate::cop::node_type::{
    BEGIN_NODE, BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, ELSE_NODE,
    IF_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE, NEXT_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, YIELD_NODE,
};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// **Round 1:** Corpus oracle reported FP=17, FN=1.
/// FP=17: Root cause was manual node traversal missing many node types. Fixed by
/// replacing with Prism visitor-based deep search. Reduced FP from 17 to 16.
///
/// **Round 2:** Corpus oracle reported FP=16, FN=1.
/// FP=16: Root cause was missing receiver check. RuboCop's `hook_block` pattern
/// uses `(send nil? :around ...)` which only matches bare `around` calls with no
/// receiver. Our code was matching `config.around { ... }` and similar calls with
/// receivers (e.g., in `spec/rails_helper.rb`, `spec/support/retry.rb`). Fixed by
/// adding `call.receiver().is_some()` early return.
///
/// FN=1: Single case in cyberark/conjur. Could not inspect source (corpus not local).
/// May be a numblock edge case or config-dependent behavior. Deferred.
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
        &[
            BEGIN_NODE,
            BLOCK_ARGUMENT_NODE,
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            ELSE_NODE,
            IF_NODE,
            LOCAL_VARIABLE_READ_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            NEXT_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
            YIELD_NODE,
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

        // Must be a bare `around` call with no receiver.
        // RuboCop's pattern is `(send nil? :around ...)` — only matches receiverless calls.
        // `config.around` (with a receiver) is NOT an RSpec hook and should not be flagged.
        if call.name().as_slice() != b"around" {
            return;
        }
        if call.receiver().is_some() {
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
                if body_uses_numbered_param_run(&block_node) || deep_contains_yield(&block_node) {
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
                // Has a block parameter — check if it's used correctly anywhere in the body.
                // RuboCop uses `def_node_search` which recursively searches ALL descendants.
                if deep_uses_param(&block_node, &name) || deep_contains_yield(&block_node) {
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
    requireds[0]
        .as_required_parameter_node()
        .map(|rp| rp.name().as_slice().to_vec())
}

/// Deep search using Prism visitor to find param usage anywhere in the block body.
/// Matches RuboCop's `def_node_search :find_arg_usage` which checks:
/// - `param.call` or `param.run`
/// - param passed as argument to any method
/// - param passed as block argument `&param`
/// - param passed to yield
fn deep_uses_param(block: &ruby_prism::BlockNode<'_>, param_name: &[u8]) -> bool {
    use ruby_prism::Visit;

    struct ParamUsageVisitor<'a> {
        param_name: &'a [u8],
        found: bool,
    }

    impl<'pr> Visit<'pr> for ParamUsageVisitor<'_> {
        fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
            if self.found {
                return;
            }

            let method = node.name().as_slice();

            // Check for param.run or param.call
            if method == b"run" || method == b"call" {
                if let Some(recv) = node.receiver() {
                    if is_param_ref(&recv, self.param_name) {
                        self.found = true;
                        return;
                    }
                }
            }

            // Check for passing param as a regular argument
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    if is_param_ref(&arg, self.param_name) {
                        self.found = true;
                        return;
                    }
                }
            }

            // Check for passing param as a block arg: `method(&param)`
            if let Some(block_arg) = node.block() {
                if let Some(ba) = block_arg.as_block_argument_node() {
                    if let Some(expr) = ba.expression() {
                        if is_param_ref(&expr, self.param_name) {
                            self.found = true;
                            return;
                        }
                    }
                }
            }

            // Continue visiting children
            ruby_prism::visit_call_node(self, node);
        }

        fn visit_yield_node(&mut self, node: &ruby_prism::YieldNode<'pr>) {
            if self.found {
                return;
            }
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    if is_param_ref(&arg, self.param_name) {
                        self.found = true;
                        return;
                    }
                }
            }
            ruby_prism::visit_yield_node(self, node);
        }
    }

    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };

    let mut visitor = ParamUsageVisitor {
        param_name,
        found: false,
    };
    visitor.visit(&body);
    visitor.found
}

/// Deep search for yield anywhere in the block body using Prism visitor.
fn deep_contains_yield(block: &ruby_prism::BlockNode<'_>) -> bool {
    use ruby_prism::Visit;

    struct YieldVisitor {
        found: bool,
    }

    impl<'pr> Visit<'pr> for YieldVisitor {
        fn visit_yield_node(&mut self, _node: &ruby_prism::YieldNode<'pr>) {
            self.found = true;
        }
    }

    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };

    let mut visitor = YieldVisitor { found: false };
    visitor.visit(&body);
    visitor.found
}

fn body_uses_numbered_param_run(block: &ruby_prism::BlockNode<'_>) -> bool {
    use ruby_prism::Visit;

    struct NumberedParamVisitor {
        found: bool,
    }

    impl<'pr> Visit<'pr> for NumberedParamVisitor {
        fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
            if self.found {
                return;
            }
            let method = node.name().as_slice();
            if method == b"run" || method == b"call" {
                if let Some(recv) = node.receiver() {
                    if let Some(rc) = recv.as_call_node() {
                        if rc.name().as_slice() == b"_1" && rc.receiver().is_none() {
                            self.found = true;
                            return;
                        }
                    }
                    if let Some(lv) = recv.as_local_variable_read_node() {
                        if lv.name().as_slice() == b"_1" {
                            self.found = true;
                            return;
                        }
                    }
                }
            }
            // Also check for _1 passed as argument or block arg
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    if let Some(lv) = arg.as_local_variable_read_node() {
                        if lv.name().as_slice() == b"_1" {
                            self.found = true;
                            return;
                        }
                    }
                }
            }
            if let Some(block_arg) = node.block() {
                if let Some(ba) = block_arg.as_block_argument_node() {
                    if let Some(expr) = ba.expression() {
                        if let Some(lv) = expr.as_local_variable_read_node() {
                            if lv.name().as_slice() == b"_1" {
                                self.found = true;
                                return;
                            }
                        }
                    }
                }
            }
            ruby_prism::visit_call_node(self, node);
        }
    }

    let body = match block.body() {
        Some(b) => b,
        None => return false,
    };

    let mut visitor = NumberedParamVisitor { found: false };
    visitor.visit(&body);
    visitor.found
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
