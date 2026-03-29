use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/ExplicitBlockArgument: Enforces explicit block argument over `yield`
/// inside a block that just passes its arguments through.
///
/// ## Investigation (2026-03)
///
/// Root causes of false positives (359 FPs, 347 from twilio-ruby):
/// - nitrocop was not checking that the block is inside a method definition
///   (`def`/`defs`). RuboCop's `on_yield` walks up to find `each_ancestor(:any_def)`
///   and skips if none is found. Blocks containing `yield` outside method defs
///   (e.g., in ERB/HAML templates, or top-level DSL code) are not flagged by RuboCop.
///
/// Root causes of false negatives (907 FNs):
/// - nitrocop required block parameters to be non-empty, missing the zero-arg case
///   (`3.times { yield }`) which RuboCop correctly flags.
///
/// Fixes applied:
/// - Switched from `check_node` to `check_source` with a visitor that tracks
///   `def_depth` to ensure blocks are inside method definitions.
/// - Added support for zero-arg blocks with zero-arg yield.
/// - Fixed FPs on destructured block params `|(key, val)|` — these are
///   MultiTargetNode, not RequiredParameterNode, and were returning None
///   which matched (None, None) in the zip comparison.
/// - Fixed FPs on blocks with `&b` parameter — `extract_block_param_names`
///   now checks for block, rest, and keyword_rest params and bails out.
///
/// Follow-up fixes:
/// - Accepted simple named rest params (`|*params| yield params`), which
///   RuboCop treats like any other block arg name match.
/// - Added LambdaNode handling for stabby lambdas (`-> { yield }`), including
///   lambdas passed as call arguments or used as a `.call` receiver. Prism
///   represents these separately from call-attached BlockNode bodies.
pub struct ExplicitBlockArgument;

impl Cop for ExplicitBlockArgument {
    fn name(&self) -> &'static str {
        "Style/ExplicitBlockArgument"
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
        let mut visitor = ExplicitBlockArgumentVisitor {
            source,
            cop: self,
            diagnostics,
            def_depth: 0,
        };
        visitor.visit(&parse_result.node());
    }
}

struct ExplicitBlockArgumentVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a ExplicitBlockArgument,
    diagnostics: &'a mut Vec<Diagnostic>,
    def_depth: u32,
}

impl<'a> ExplicitBlockArgumentVisitor<'a> {
    /// Check if a block-like body is a sole `yield` forwarding its parameters unchanged.
    /// `start_offset` is the offense location start: for call-attached blocks this is the
    /// outer call start, while for stabby lambdas it is the lambda literal start.
    fn check_yielding_block_like(
        &mut self,
        body: Option<ruby_prism::Node<'_>>,
        parameters: Option<ruby_prism::Node<'_>>,
        start_offset: usize,
    ) {
        // Must be inside a method definition
        if self.def_depth == 0 {
            return;
        }

        // Must have a body
        let stmts = match body.and_then(|b| b.as_statements_node()) {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().into_iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        // Single statement must be a yield
        let yield_node = match body_nodes[0].as_yield_node() {
            Some(y) => y,
            None => return,
        };

        // Get block params (may be empty for zero-arg blocks like `{ yield }`)
        // Returns None if params are not a simple RuboCop-compatible forwarding shape.
        let block_param_names = match self.extract_block_param_names(parameters) {
            Some(names) => names,
            None => return,
        };

        // Get yield args (None if any arg is not a simple local variable read)
        let yield_arg_names = match self.extract_yield_arg_names(&yield_node) {
            Some(names) => names,
            None => return,
        };

        // Both must have same count
        if block_param_names.len() != yield_arg_names.len() {
            return;
        }

        // Each yield arg must match the corresponding block param
        for (param, arg) in block_param_names.iter().zip(yield_arg_names.iter()) {
            if param != arg {
                return;
            }
        }

        // Report the offense at the full call+block expression
        let (line, column) = self.source.offset_to_line_col(start_offset);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Consider using explicit block argument in the surrounding method's signature over `yield`.".to_string(),
        ));
    }

    /// Extract block parameter names as a list of byte slices.
    /// Returns `Some(vec![])` for blocks with no parameters.
    /// Returns `None` if params have shapes this cop still intentionally skips
    /// (destructured, optional/post/keyword args, block args, or anonymous rest args).
    fn extract_block_param_names(
        &self,
        parameters: Option<ruby_prism::Node<'_>>,
    ) -> Option<Vec<Vec<u8>>> {
        let params = match parameters {
            Some(p) => p,
            None => return Some(vec![]),
        };

        let block_params = match params.as_block_parameters_node() {
            Some(p) => p,
            None => return Some(vec![]),
        };

        let params_node = match block_params.parameters() {
            Some(p) => p,
            None => return Some(vec![]),
        };

        // Keep the matcher narrow: simple required params, optionally followed by a
        // named `*rest` param. RuboCop accepts `|*params| yield params`, but the other
        // parameter kinds below are not covered by our current fixture/corpus evidence.
        if params_node.block().is_some()
            || !params_node.optionals().is_empty()
            || !params_node.posts().is_empty()
            || !params_node.keywords().is_empty()
            || params_node.keyword_rest().is_some()
        {
            return None;
        }

        let mut names = Vec::new();
        for p in params_node.requireds().into_iter() {
            match p.as_required_parameter_node() {
                Some(rp) => names.push(rp.name().as_slice().to_vec()),
                // Destructured param like |(key, val)| — not simple
                None => return None,
            }
        }

        if let Some(rest) = params_node.rest() {
            let rest_param = rest.as_rest_parameter_node()?;
            let name = rest_param.name()?;
            names.push(name.as_slice().to_vec());
        }

        Some(names)
    }

    /// Extract yield argument names (must all be local variable reads).
    /// Returns empty vec for bare `yield`.
    /// Returns `None` if any argument is not a simple local variable read.
    fn extract_yield_arg_names(
        &self,
        yield_node: &ruby_prism::YieldNode<'_>,
    ) -> Option<Vec<Vec<u8>>> {
        let args = match yield_node.arguments() {
            Some(a) => a,
            None => return Some(vec![]),
        };

        let mut names = Vec::new();
        for a in args.arguments().into_iter() {
            match a.as_local_variable_read_node() {
                Some(lv) => names.push(lv.name().as_slice().to_vec()),
                None => return None,
            }
        }
        Some(names)
    }
}

impl<'a, 'pr> Visit<'pr> for ExplicitBlockArgumentVisitor<'a> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.def_depth += 1;
        ruby_prism::visit_def_node(self, node);
        self.def_depth -= 1;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this call has a block that just yields
        if let Some(block_arg) = node.block() {
            if let Some(block) = block_arg.as_block_node() {
                self.check_yielding_block_like(
                    block.body(),
                    block.parameters(),
                    node.location().start_offset(),
                );
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_forwarding_super_node(&mut self, node: &ruby_prism::ForwardingSuperNode<'pr>) {
        // `super { yield }` (no explicit args) parses as ForwardingSuperNode
        if let Some(block) = node.block() {
            self.check_yielding_block_like(
                block.body(),
                block.parameters(),
                node.location().start_offset(),
            );
        }
        ruby_prism::visit_forwarding_super_node(self, node);
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        // `super(args) { yield }` parses as SuperNode; block() returns Node
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                self.check_yielding_block_like(
                    block_node.body(),
                    block_node.parameters(),
                    node.location().start_offset(),
                );
            }
        }
        ruby_prism::visit_super_node(self, node);
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.check_yielding_block_like(
            node.body(),
            node.parameters(),
            node.location().start_offset(),
        );
        ruby_prism::visit_lambda_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExplicitBlockArgument, "cops/style/explicit_block_argument");
}
