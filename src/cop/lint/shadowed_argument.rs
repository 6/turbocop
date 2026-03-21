/// Lint/ShadowedArgument: checks for method/block arguments that are reassigned
/// before being used.
///
/// ## Investigation findings
///
/// FP root cause: nitrocop did not check whether the argument was ever referenced
/// (read) in the body. RuboCop's VariableForce checks `argument.referenced?` and
/// skips unreferenced arguments. Without this, `def foo(x); x = 42; end` was
/// flagged even though `x` is never read.
///
/// FN root cause: nitrocop only scanned top-level body statements for assignments.
/// RuboCop does a deep scan of ALL assignments in the scope (including those nested
/// inside conditionals, blocks, and lambdas). When a conditional/block assignment
/// precedes an unconditional one, RuboCop reports at the declaration node (location
/// unknown). Nitrocop missed these patterns entirely because it bailed out on
/// encountering a conditional at the top level.
///
/// Additional FN: shorthand assignments (`||=`, `+=`) should stop the scan (the
/// argument is used) but should not prevent detecting a later unconditional
/// reassignment. The old code returned immediately on shorthand, which could miss
/// a subsequent shadowing write.
///
/// Additional FN: `value = super` was treated as "RHS references arg" because
/// `ForwardingSuperNode` unconditionally counted as a reference. RuboCop's
/// `uses_var?` only matches `(lvar %)`, so bare `super` does NOT count.
/// Split into `node_references_local_explicit` (no super) for RHS checks.
///
/// Additional FN: nested blocks/defs inside outer defs/blocks were never visited
/// because `visit_def_node`/`visit_block_node`/`visit_lambda_node` did not recurse
/// into their bodies. Added explicit recursion after checking parameters.
///
/// FP fix: multi-write `a, b, arg = super` was flagged because
/// `node_references_local_explicit` (used for RHS checks) does not count
/// `ForwardingSuperNode` as a reference. But bare `super` implicitly forwards
/// ALL method arguments, so the param IS used on the RHS. Fixed by checking
/// `node.value().as_forwarding_super_node().is_some()` in `visit_multi_write_node`
/// before falling through to `node_references_local_explicit`.
///
/// Additional FN (5 corpus): Three root causes:
/// 1. `collect_param_names`/`find_param_offset` did not handle `BlockParameterNode`
///    (`&block` params), causing block-pass args to be invisible to the cop entirely.
///    (chefspec FN, seeing_is_believing FN)
/// 2. `AssignmentCollector` did not handle `MultiWriteNode` (parallel/destructuring
///    assignment like `a, b = expr`). `LocalVariableTargetNode` targets inside
///    multi-writes were never collected as assignments. (xiki FN x2, brakeman FN)
/// 3. The `&&` short-circuit case (`char && block = lambda { ... }`) was already
///    handled by default visitor recursion into `AndNode`; the actual blocker was
///    cause #1 (`&block` not collected).
///
/// ## Additional FN Investigation (2026-03-21)
///
/// The 5 FN cases in the corpus report (watch, regex, replace, page, initialize)
/// were investigated:
///
/// 1. `watch` and `page` cases: The assignments (`options, individual_files = ...`
///    and `options, page = ...`) are inside if-modifier conditionals. RuboCop's
///    `conditional_assignment?` returns true for these, causing the assignment to be
///    skipped. Nitrocop's `conditional_depth > 0` also skips these. Both
///    implementations agree these are not reported.
///
/// 2. `regex`/`replace` cases: The FN report claims `options` is a parameter
///    being shadowed, but the method signature `def regex(text, pattern, flags)`
///    has no `options` parameter. The `options = 0` assignment creates a local
///    variable, not shadowing a parameter. FN report appears to be in error.
///
/// 3. `initialize` case: The assignment `y = x[:y]` has `y` on the LHS and `x[:y]`
///    on the RHS. The `y` in `x[:y]` is a method argument (symbol key), not a
///    local variable read. Since `y` is never referenced as a local variable,
///    `argument.referenced?` is false and no offense is reported. Both RuboCop
///    and nitrocop correctly skip this case.
///
/// Conclusion: The 5 FN cases are either not RuboCop offenses (conditional
/// assignments, parameter not referenced) or contain errors in the FN report.
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ShadowedArgument;

impl Cop for ShadowedArgument {
    fn name(&self) -> &'static str {
        "Lint/ShadowedArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let ignore_implicit = config.get_bool("IgnoreImplicitReferences", false);
        let mut visitor = ShadowedArgVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            ignore_implicit,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ShadowedArgVisitor<'a, 'src> {
    cop: &'a ShadowedArgument,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    ignore_implicit: bool,
}

/// Extract parameter names from a ParametersNode.
fn collect_param_names(params: &ruby_prism::ParametersNode<'_>) -> Vec<Vec<u8>> {
    let mut names = Vec::new();
    for req in params.requireds().iter() {
        if let Some(rp) = req.as_required_parameter_node() {
            names.push(rp.name().as_slice().to_vec());
        }
    }
    for opt in params.optionals().iter() {
        if let Some(op) = opt.as_optional_parameter_node() {
            names.push(op.name().as_slice().to_vec());
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rp) = rest.as_rest_parameter_node() {
            if let Some(name) = rp.name() {
                names.push(name.as_slice().to_vec());
            }
        }
    }
    for kw in params.keywords().iter() {
        if let Some(kp) = kw.as_required_keyword_parameter_node() {
            names.push(kp.name().as_slice().to_vec());
        }
        if let Some(kp) = kw.as_optional_keyword_parameter_node() {
            names.push(kp.name().as_slice().to_vec());
        }
    }
    if let Some(block) = params.block() {
        if let Some(name) = block.name() {
            names.push(name.as_slice().to_vec());
        }
    }
    names
}

/// Information about an assignment to a parameter found during deep scan.
#[derive(Debug)]
struct ParamAssignment {
    /// Byte offset of the assignment node start.
    offset: usize,
    /// Whether the RHS of the assignment references the parameter.
    rhs_uses_param: bool,
    /// Whether this is a shorthand assignment (||=, &&=, +=, etc.).
    is_shorthand: bool,
    /// Whether the assignment is inside a conditional, block, or rescue
    /// (i.e., may not always execute).
    is_conditional: bool,
}

/// Collect all assignments to `param_name` in the body, including nested ones.
/// `scope_node` is the def/block/lambda node that defines the scope boundary.
fn collect_assignments(body: &ruby_prism::Node<'_>, param_name: &[u8]) -> Vec<ParamAssignment> {
    let mut collector = AssignmentCollector {
        param_name: param_name.to_vec(),
        assignments: Vec::new(),
        conditional_depth: 0,
    };
    collector.visit(body);
    // Sort by offset for ordered processing
    collector.assignments.sort_by_key(|a| a.offset);
    collector.assignments
}

struct AssignmentCollector {
    param_name: Vec<u8>,
    assignments: Vec<ParamAssignment>,
    /// Depth inside conditional/block/rescue constructs.
    conditional_depth: usize,
}

impl<'pr> Visit<'pr> for AssignmentCollector {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.param_name.as_slice() {
            let rhs_uses_param = node_references_local_explicit(&node.value(), &self.param_name);
            self.assignments.push(ParamAssignment {
                offset: node.location().start_offset(),
                rhs_uses_param,
                is_shorthand: false,
                is_conditional: self.conditional_depth > 0,
            });
        }
        // Visit children (the value node)
        self.visit(&node.value());
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.param_name.as_slice() {
            self.assignments.push(ParamAssignment {
                offset: node.location().start_offset(),
                rhs_uses_param: true, // always uses param
                is_shorthand: true,
                is_conditional: self.conditional_depth > 0,
            });
        }
        self.visit(&node.value());
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.param_name.as_slice() {
            self.assignments.push(ParamAssignment {
                offset: node.location().start_offset(),
                rhs_uses_param: true,
                is_shorthand: true,
                is_conditional: self.conditional_depth > 0,
            });
        }
        self.visit(&node.value());
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.param_name.as_slice() {
            self.assignments.push(ParamAssignment {
                offset: node.location().start_offset(),
                rhs_uses_param: true,
                is_shorthand: true,
                is_conditional: self.conditional_depth > 0,
            });
        }
        self.visit(&node.value());
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        // Check if any LHS target matches the param name.
        // Bare `super` (ForwardingSuperNode) implicitly forwards all arguments,
        // so treat it as referencing the param to avoid FP on `a, b, arg = super`.
        let rhs_uses_param = node.value().as_forwarding_super_node().is_some()
            || node_references_local_explicit(&node.value(), &self.param_name);
        for target in node.lefts().iter() {
            if let Some(local) = target.as_local_variable_target_node() {
                if local.name().as_slice() == self.param_name.as_slice() {
                    self.assignments.push(ParamAssignment {
                        offset: local.location().start_offset(),
                        rhs_uses_param,
                        is_shorthand: false,
                        is_conditional: self.conditional_depth > 0,
                    });
                }
            }
        }
        if let Some(rest) = node.rest() {
            if let Some(splat) = rest.as_splat_node() {
                if let Some(expr) = splat.expression() {
                    if let Some(local) = expr.as_local_variable_target_node() {
                        if local.name().as_slice() == self.param_name.as_slice() {
                            self.assignments.push(ParamAssignment {
                                offset: local.location().start_offset(),
                                rhs_uses_param,
                                is_shorthand: false,
                                is_conditional: self.conditional_depth > 0,
                            });
                        }
                    }
                }
            }
        }
        for target in node.rights().iter() {
            if let Some(local) = target.as_local_variable_target_node() {
                if local.name().as_slice() == self.param_name.as_slice() {
                    self.assignments.push(ParamAssignment {
                        offset: local.location().start_offset(),
                        rhs_uses_param,
                        is_shorthand: false,
                        is_conditional: self.conditional_depth > 0,
                    });
                }
            }
        }
        // Visit the RHS value to find any nested assignments
        self.visit(&node.value());
    }

    // Conditionals increase depth
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_if_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_unless_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_case_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_case_match_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_while_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_until_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_rescue_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        // begin/rescue: the rescue part is conditional
        self.conditional_depth += 1;
        ruby_prism::visit_begin_node(self, node);
        self.conditional_depth -= 1;
    }

    // Blocks and lambdas are conditional (may not execute)
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_block_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_lambda_node(self, node);
        self.conditional_depth -= 1;
    }

    // Don't cross scope boundaries
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

/// Collect all reference (read) positions for a local variable in the body.
/// Returns byte offsets of each read. Does not include reads that are part
/// of the RHS of assignments to the same variable (those are tracked separately).
fn collect_reference_offsets(
    body: &ruby_prism::Node<'_>,
    param_name: &[u8],
    ignore_implicit: bool,
) -> Vec<usize> {
    let mut collector = RefCollector {
        param_name: param_name.to_vec(),
        offsets: Vec::new(),
        ignore_implicit,
    };
    collector.visit(body);
    collector.offsets
}

struct RefCollector {
    param_name: Vec<u8>,
    offsets: Vec<usize>,
    ignore_implicit: bool,
}

impl<'pr> Visit<'pr> for RefCollector {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.param_name.as_slice() {
            self.offsets.push(node.location().start_offset());
        }
    }

    fn visit_forwarding_super_node(&mut self, node: &ruby_prism::ForwardingSuperNode<'pr>) {
        if !self.ignore_implicit {
            self.offsets.push(node.location().start_offset());
        }
    }

    // Don't cross scope boundaries
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

impl ShadowedArgVisitor<'_, '_> {
    fn check_one_param(
        &mut self,
        param_name: &[u8],
        param_decl_offset: usize,
        body: &ruby_prism::Node<'_>,
    ) {
        // Collect all references (reads) to the param in the body
        let ref_offsets = collect_reference_offsets(body, param_name, self.ignore_implicit);

        // RuboCop: `return unless argument.referenced?`
        // If the argument is never referenced at all, no offense.
        if ref_offsets.is_empty() {
            return;
        }

        // Collect all assignments to the param (deep scan)
        let assignments = collect_assignments(body, param_name);
        if assignments.is_empty() {
            return;
        }

        // Walk assignments in order, mirroring RuboCop's
        // `assignment_without_argument_usage` reduce logic.
        let mut location_known = true;

        for asgn in &assignments {
            // Shorthand assignments always use the argument
            if asgn.is_shorthand {
                location_known = false;
                continue;
            }

            // If the RHS uses the param, this is not shadowing (e.g., foo = foo + 1)
            if asgn.rhs_uses_param {
                // location remains known for subsequent assignments
                continue;
            }

            // This assignment doesn't use the param on RHS.
            if asgn.is_conditional {
                // Inside a conditional/block: can't tell if it executes.
                // Mark location as unknown and continue looking.
                location_known = false;
                continue;
            }

            // Unconditional assignment that doesn't use the param.
            // Before flagging: check if there's a reference before this assignment.
            let assignment_pos = asgn.offset;
            let has_prior_ref = ref_offsets.iter().any(|&ref_pos| ref_pos <= assignment_pos);
            if has_prior_ref {
                return;
            }

            // This is a shadowing assignment.
            if location_known {
                // Report at the assignment location
                let (line, column) = self.source.offset_to_line_col(assignment_pos);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Argument `{}` was shadowed by a local variable before it was used.",
                        String::from_utf8_lossy(param_name)
                    ),
                ));
            } else {
                // Report at the declaration node (location unknown)
                let (line, column) = self.source.offset_to_line_col(param_decl_offset);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Argument `{}` was shadowed by a local variable before it was used.",
                        String::from_utf8_lossy(param_name)
                    ),
                ));
            }
            return;
        }
    }
}

/// Check if a node tree contains an explicit local variable read of the given name.
/// This does NOT count `super` (ForwardingSuperNode) as a reference, because
/// RuboCop's `uses_var?` (which checks RHS of assignments) only looks for `(lvar %)`.
fn node_references_local_explicit(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    let mut finder = LocalRefFinder {
        name: name.to_vec(),
        found: false,
        include_super: false,
    };
    finder.visit(node);
    finder.found
}

struct LocalRefFinder {
    name: Vec<u8>,
    found: bool,
    include_super: bool,
}

impl<'pr> Visit<'pr> for LocalRefFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.name.as_slice() {
            self.found = true;
        }
    }

    fn visit_forwarding_super_node(&mut self, _node: &ruby_prism::ForwardingSuperNode<'pr>) {
        if self.include_super {
            self.found = true;
        }
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

/// Find the byte offset of a parameter name within a ParametersNode.
fn find_param_offset(params: &ruby_prism::ParametersNode<'_>, name: &[u8]) -> Option<usize> {
    for req in params.requireds().iter() {
        if let Some(rp) = req.as_required_parameter_node() {
            if rp.name().as_slice() == name {
                return Some(rp.location().start_offset());
            }
        }
    }
    for opt in params.optionals().iter() {
        if let Some(op) = opt.as_optional_parameter_node() {
            if op.name().as_slice() == name {
                return Some(op.location().start_offset());
            }
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rp) = rest.as_rest_parameter_node() {
            if let Some(pname) = rp.name() {
                if pname.as_slice() == name {
                    return Some(rp.location().start_offset());
                }
            }
        }
    }
    for kw in params.keywords().iter() {
        if let Some(kp) = kw.as_required_keyword_parameter_node() {
            if kp.name().as_slice() == name {
                return Some(kp.location().start_offset());
            }
        }
        if let Some(kp) = kw.as_optional_keyword_parameter_node() {
            if kp.name().as_slice() == name {
                return Some(kp.location().start_offset());
            }
        }
    }
    if let Some(block) = params.block() {
        if let Some(pname) = block.name() {
            if pname.as_slice() == name {
                return Some(block.location().start_offset());
            }
        }
    }
    None
}

impl<'pr> Visit<'pr> for ShadowedArgVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(params) = node.parameters() {
            let names = collect_param_names(&params);
            for name in &names {
                if let (Some(body), Some(decl_offset)) =
                    (node.body(), find_param_offset(&params, name))
                {
                    self.check_one_param(name, decl_offset, &body);
                }
            }
        }
        // Recurse into the body to find nested blocks/defs/lambdas
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        if let Some(params_node) = node.parameters() {
            if let Some(bp) = params_node.as_block_parameters_node() {
                if let Some(inner) = bp.parameters() {
                    let names = collect_param_names(&inner);
                    for name in &names {
                        if let (Some(body), Some(decl_offset)) =
                            (node.body(), find_param_offset(&inner, name))
                        {
                            self.check_one_param(name, decl_offset, &body);
                        }
                    }
                }
            }
        }
        // Recurse into the body to find nested blocks/defs/lambdas
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if let Some(params_node) = node.parameters() {
            if let Some(bp) = params_node.as_block_parameters_node() {
                if let Some(inner) = bp.parameters() {
                    let names = collect_param_names(&inner);
                    for name in &names {
                        if let (Some(body), Some(decl_offset)) =
                            (node.body(), find_param_offset(&inner, name))
                        {
                            self.check_one_param(name, decl_offset, &body);
                        }
                    }
                }
            }
        }
        // Recurse into the body to find nested blocks/defs/lambdas
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ShadowedArgument, "cops/lint/shadowed_argument");
}
