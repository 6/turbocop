use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-07)
///
/// FP=0, FN=3 fixed. Root cause: RuboCop's `shadowed_block_argument?` only detects
/// shadowing when the entire method body is a single block expression. If the method
/// has multiple statements, the body is a `begin` node and the check returns false,
/// so RuboCop still flags `block.call` inside inner blocks that shadow the param name.
/// Our previous implementation was more correct (detected shadowing in all inner blocks)
/// but caused FNs vs RuboCop. Now we match RuboCop's limited shadowing check exactly.
///
/// ## Extended corpus FP fix (2026-03-22)
///
/// 1 FP in extended corpus (Albacore__albacore repo, `lib/albacore/cross_platform_cmd.rb:111`).
/// Root cause: RuboCop's `calls_to_report` uses `return []` (not `next`) inside `map`
/// when checking `args_include_block_pass?`. This means if ANY `block.call(...)` in the
/// method has a `&block_pass` argument, ALL offenses for that method are suppressed.
/// Our implementation was checking per-call (`node.block().is_none()`) which only
/// suppressed the specific call with the block argument. Fixed by adding a
/// `BlockPassFinder` pre-scan that checks if any `block.call` in the method body has
/// a block argument (block_pass or block literal), and if so, skips all reporting
/// for that method. Pending corpus confirmation.
pub struct RedundantBlockCall;

impl Cop for RedundantBlockCall {
    fn name(&self) -> &'static str {
        "Performance/RedundantBlockCall"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = DefVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct DefVisitor<'a, 'src> {
    cop: &'a RedundantBlockCall,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for DefVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        check_def(self.cop, self.source, node, &mut self.diagnostics);
        // Continue recursing into nested defs (they have their own scope,
        // handled by BlockCallFinder not descending into defs)
        ruby_prism::visit_def_node(self, node);
    }
}

/// Check a def node for a &blockarg parameter and block.call usage.
fn check_def(
    cop: &RedundantBlockCall,
    source: &SourceFile,
    def_node: &ruby_prism::DefNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Look for a &blockarg parameter
    let params = match def_node.parameters() {
        Some(p) => p,
        None => return,
    };

    let blockarg = match params.block() {
        Some(b) => b,
        None => return,
    };

    let blockarg_name = match blockarg.name() {
        Some(n) => n,
        None => return,
    };

    let arg_name = blockarg_name.as_slice();

    // Now look for <arg_name>.call in the body
    let body = match def_node.body() {
        Some(b) => b,
        None => return,
    };

    // RuboCop's shadowed_block_argument? — only detects shadowing when the
    // entire method body is a single block expression (body.block_type?).
    // If the method has multiple statements, the shadowing is NOT detected.
    if let Some(stmts) = body.as_statements_node() {
        let stmts_vec: Vec<_> = stmts.body().iter().collect();
        if stmts_vec.len() == 1 {
            if let Some(call) = stmts_vec[0].as_call_node() {
                if let Some(block) = call.block() {
                    if let Some(block_node) = block.as_block_node() {
                        if block_params_include(&block_node, arg_name) {
                            return;
                        }
                    }
                }
            }
        }
    }

    // Check if the block arg is reassigned in the body — if so, skip
    let mut reassign_finder = ReassignFinder {
        name: arg_name,
        found: false,
    };
    reassign_finder.visit(&body);
    if reassign_finder.found {
        return;
    }

    // RuboCop's calls_to_report uses `return []` inside `map` when ANY block.call
    // has a block_pass argument — this suppresses ALL offenses for the entire method.
    // Pre-scan: if any block.call has a block argument (block_pass or block literal),
    // skip all reporting for this method.
    let mut block_pass_finder = BlockPassFinder {
        name: arg_name,
        found: false,
    };
    block_pass_finder.visit(&body);
    if block_pass_finder.found {
        return;
    }

    let mut call_finder = BlockCallFinder {
        cop,
        source,
        arg_name,
        diagnostics,
    };
    call_finder.visit(&body);
}

struct ReassignFinder<'a> {
    name: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for ReassignFinder<'_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }
}

/// Pre-scan visitor: checks if any `block.call(...)` has a block argument
/// (block_pass like `&proc` or block literal like `{ ... }`).
/// RuboCop suppresses ALL offenses for the method if any call has one.
struct BlockPassFinder<'a> {
    name: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for BlockPassFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if !self.found && node.name().as_slice() == b"call" && node.block().is_some() {
            if let Some(recv) = node.receiver() {
                if let Some(local_var) = recv.as_local_variable_read_node() {
                    if local_var.name().as_slice() == self.name {
                        self.found = true;
                        return;
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Don't descend into nested defs
    }
}

struct BlockCallFinder<'a, 'src, 'd> {
    cop: &'a RedundantBlockCall,
    source: &'src SourceFile,
    arg_name: &'a [u8],
    diagnostics: &'d mut Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for BlockCallFinder<'_, '_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"call" {
            // Skip safe navigation (&.call) — yield doesn't have nil-safe semantics
            let is_safe_nav = node
                .call_operator_loc()
                .is_some_and(|op| op.as_slice() == b"&.");
            if !is_safe_nav {
                if let Some(recv) = node.receiver() {
                    if let Some(local_var) = recv.as_local_variable_read_node() {
                        if local_var.name().as_slice() == self.arg_name {
                            // Don't flag if the call has any block argument
                            // (block literal or &block_pass)
                            if node.block().is_none() {
                                let loc = node.location();
                                let (line, column) =
                                    self.source.offset_to_line_col(loc.start_offset());
                                let msg = format!(
                                    "Use `yield` instead of `{}.call`.",
                                    std::str::from_utf8(self.arg_name).unwrap_or("block")
                                );
                                self.diagnostics.push(self.cop.diagnostic(
                                    self.source,
                                    line,
                                    column,
                                    msg,
                                ));
                            }
                        }
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Don't descend into nested def nodes (they have their own scope)
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Visit all blocks normally — RuboCop's shadowed_block_argument? only checks
        // at the top level (single-block method body), not inner blocks.
        ruby_prism::visit_block_node(self, node);
    }
}

fn block_params_include(block: &ruby_prism::BlockNode<'_>, name: &[u8]) -> bool {
    if let Some(params) = block.parameters() {
        if let Some(bp) = params.as_block_parameters_node() {
            if let Some(inner) = bp.parameters() {
                for req in inner.requireds().iter() {
                    if let Some(rp) = req.as_required_parameter_node() {
                        if rp.name().as_slice() == name {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantBlockCall, "cops/performance/redundant_block_call");
}
