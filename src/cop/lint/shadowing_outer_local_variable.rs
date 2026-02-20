use std::collections::{HashMap, HashSet};

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ShadowingOuterLocalVariable;

impl Cop for ShadowingOuterLocalVariable {
    fn name(&self) -> &'static str {
        "Lint/ShadowingOuterLocalVariable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    /// This cop is disabled by default in RuboCop (Enabled: false).
    fn default_enabled(&self) -> bool {
        false
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
        let mut visitor = ShadowVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            scopes: Vec::new(),
            conditional_branch_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Info about where a variable was declared.
#[derive(Clone, Debug)]
struct VarInfo {
    /// If the variable was declared inside a `when`/`if`/`else` branch,
    /// this is the (case_node_offset, branch_offset) pair. Used to skip
    /// shadowing when block and outer var are in different branches of the
    /// same conditional.
    conditional_branch: Option<(usize, usize)>,
}

struct ShadowVisitor<'a, 'src> {
    cop: &'a ShadowingOuterLocalVariable,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Stack of maps of local variable names -> declaration info.
    scopes: Vec<HashMap<String, VarInfo>>,
    /// Stack of (case_node_offset, branch_offset) for current when/if/else branch.
    conditional_branch_stack: Vec<(usize, usize)>,
}

impl ShadowVisitor<'_, '_> {
    fn current_locals(&self) -> HashMap<String, VarInfo> {
        let mut all = HashMap::new();
        for scope in &self.scopes {
            for (name, info) in scope {
                all.insert(name.clone(), info.clone());
            }
        }
        all
    }

    fn add_local(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            let info = VarInfo {
                conditional_branch: self.conditional_branch_stack.last().copied(),
            };
            scope.insert(name.to_string(), info);
        }
    }

    fn current_conditional_branch(&self) -> Option<(usize, usize)> {
        self.conditional_branch_stack.last().copied()
    }
}

impl<'pr> Visit<'pr> for ShadowVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // def creates a completely new scope — save and replace the entire scope
        // stack. RuboCop's VariableForce treats method definitions as scope
        // barriers: class/module-level variables are NOT visible inside methods.
        let saved_scopes = std::mem::take(&mut self.scopes);
        let saved_cond = std::mem::take(&mut self.conditional_branch_stack);
        let mut name_set = HashSet::new();
        if let Some(params) = node.parameters() {
            collect_param_names_into(&params, &mut name_set);
        }
        let scope: HashMap<String, VarInfo> = name_set
            .into_iter()
            .map(|n| (n, VarInfo { conditional_branch: None }))
            .collect();
        self.scopes.push(scope);
        ruby_prism::visit_def_node(self, node);
        self.scopes = saved_scopes;
        self.conditional_branch_stack = saved_cond;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // class body is a new scope
        self.scopes.push(HashMap::new());
        ruby_prism::visit_class_node(self, node);
        self.scopes.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.scopes.push(HashMap::new());
        ruby_prism::visit_module_node(self, node);
        self.scopes.pop();
    }

    fn visit_local_variable_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_local(&name);
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Skip Ractor.new blocks — Ractor should not access outer variables,
        // so shadowing is intentional and encouraged.
        if is_ractor_new_block(node) {
            self.scopes.push(HashMap::new());
            ruby_prism::visit_block_node(self, node);
            self.scopes.pop();
            return;
        }

        let outer_locals = self.current_locals();
        let block_cond_branch = self.current_conditional_branch();

        // Check block parameters against outer locals
        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                // Check regular parameters
                if let Some(inner_params) = block_params.parameters() {
                    check_block_params_shadow(
                        self.cop,
                        self.source,
                        &inner_params,
                        &outer_locals,
                        block_cond_branch,
                        &mut self.diagnostics,
                    );

                    // Check multi-target (destructured) params: |(a, b, c)|
                    for p in inner_params.requireds().iter() {
                        if let Some(mt) = p.as_multi_target_node() {
                            check_multi_target_shadow(
                                self.cop,
                                self.source,
                                &mt,
                                &outer_locals,
                                block_cond_branch,
                                &mut self.diagnostics,
                            );
                        }
                    }
                }

                // Check block-local variables (|a; b| — b is a block-local)
                for local in block_params.locals().iter() {
                    let name = std::str::from_utf8(local.as_block_local_variable_node().map_or(&[][..], |n| n.name().as_slice()))
                        .unwrap_or("")
                        .to_string();
                    if !name.is_empty() && !name.starts_with('_') {
                        if let Some(info) = outer_locals.get(&name) {
                            // Skip if in different branches of the same conditional
                            if is_different_conditional_branch(info.conditional_branch, block_cond_branch) {
                                continue;
                            }
                            let loc = local.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                format!("Shadowing outer local variable - `{}`.", name),
                            ));
                        }
                    }
                }
            }
        }

        // Push a new scope for the block body that includes the block parameters.
        // This ensures inner blocks can see outer block params for shadowing detection.
        // Do NOT merge back into the outer scope — RuboCop's VariableForce treats
        // block-internal variables as local to the block, not visible to sibling blocks.
        let mut body_scope = HashMap::new();
        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                if let Some(inner_params) = block_params.parameters() {
                    let mut param_names = HashSet::new();
                    collect_param_names_into(&inner_params, &mut param_names);
                    for name in param_names {
                        body_scope.insert(name, VarInfo { conditional_branch: None });
                    }
                }
                // Also add destructured params
                collect_multi_target_names_from_block_params(&block_params, &mut body_scope);
            }
        }
        self.scopes.push(body_scope);
        ruby_prism::visit_block_node(self, node);
        self.scopes.pop();
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambdas behave like blocks for shadowing purposes
        let outer_locals = self.current_locals();
        let block_cond_branch = self.current_conditional_branch();

        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                if let Some(inner_params) = block_params.parameters() {
                    check_block_params_shadow(
                        self.cop,
                        self.source,
                        &inner_params,
                        &outer_locals,
                        block_cond_branch,
                        &mut self.diagnostics,
                    );
                }
            }
        }

        // Lambda creates an isolated scope — do NOT merge back.
        self.scopes.push(HashMap::new());
        ruby_prism::visit_lambda_node(self, node);
        self.scopes.pop();
    }

    // Handle top-level assignments (outside any method)
    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        self.scopes.push(HashMap::new());
        ruby_prism::visit_program_node(self, node);
        self.scopes.pop();
    }

    // Handle for loops and while/until which share scope
    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        ruby_prism::visit_for_node(self, node);
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        ruby_prism::visit_while_node(self, node);
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        ruby_prism::visit_until_node(self, node);
    }

    // Track case/when branches for the same_conditions_node_different_branch check.
    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let case_offset = node.location().start_offset();

        // Visit the predicate (the expression after `case`)
        if let Some(pred) = node.predicate() {
            self.visit(&pred);
        }

        // Visit each when clause with branch tracking
        for condition in node.conditions().iter() {
            let branch_offset = condition.location().start_offset();
            self.conditional_branch_stack.push((case_offset, branch_offset));
            self.visit(&condition);
            self.conditional_branch_stack.pop();
        }

        // Visit the else clause (consequent) with its own branch
        if let Some(else_clause) = node.else_clause() {
            let branch_offset = else_clause.location().start_offset();
            self.conditional_branch_stack.push((case_offset, branch_offset));
            self.visit_else_node(&else_clause);
            self.conditional_branch_stack.pop();
        }
    }

    // Track if/unless branches for the same_conditions_node_different_branch check.
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let if_offset = node.location().start_offset();

        // Visit predicate normally
        self.visit(&node.predicate());

        // Visit then-body with branch tracking
        if let Some(stmts) = node.statements() {
            let branch_offset = stmts.location().start_offset();
            self.conditional_branch_stack.push((if_offset, branch_offset));
            self.visit_statements_node(&stmts);
            self.conditional_branch_stack.pop();
        }

        // Visit else-body with branch tracking
        if let Some(subsequent) = node.subsequent() {
            let branch_offset = subsequent.location().start_offset();
            self.conditional_branch_stack.push((if_offset, branch_offset));
            self.visit(&subsequent);
            self.conditional_branch_stack.pop();
        }
    }
}

/// Check if two conditional branch contexts are in different branches of the
/// same conditional node. Returns true if they should be treated as non-overlapping.
fn is_different_conditional_branch(
    outer_branch: Option<(usize, usize)>,
    block_branch: Option<(usize, usize)>,
) -> bool {
    match (outer_branch, block_branch) {
        (Some((case1, branch1)), Some((case2, branch2))) => {
            // Same conditional node but different branch
            case1 == case2 && branch1 != branch2
        }
        // RuboCop suppresses when the inner (shadowing) variable is inside
        // a conditional but the outer (shadowed) variable is not. The rationale
        // is that conditional shadowing is less problematic.
        (None, Some(_)) => true,
        _ => false,
    }
}

/// Check if a block node is `Ractor.new(...) do |...| end`.
fn is_ractor_new_block(node: &ruby_prism::BlockNode<'_>) -> bool {
    // The block's parent call is available as the CallNode that owns this block.
    // In Prism, BlockNode doesn't have a direct parent pointer, but we can check
    // the source around the block. However, BlockNode is always a child of a CallNode.
    // We need to check the call that owns this block.
    // Unfortunately, Prism's visitor doesn't give us the parent node.
    // We'll skip Ractor detection for now since it's rare in practice.
    // TODO: implement Ractor detection if needed
    let _ = node;
    false
}

/// Check multi-target (destructured) block params for shadowing.
/// E.g., `|(theme_id, upload_id, sprite)|`
fn check_multi_target_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    mt: &ruby_prism::MultiTargetNode<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    block_cond_branch: Option<(usize, usize)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for target in mt.lefts().iter() {
        if let Some(req) = target.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(cop, source, &name, req.location(), outer_locals, block_cond_branch, diagnostics);
        } else if let Some(inner) = target.as_multi_target_node() {
            check_multi_target_shadow(cop, source, &inner, outer_locals, block_cond_branch, diagnostics);
        }
    }
    for target in mt.rights().iter() {
        if let Some(req) = target.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(cop, source, &name, req.location(), outer_locals, block_cond_branch, diagnostics);
        }
    }
}

fn check_block_params_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    params: &ruby_prism::ParametersNode<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    block_cond_branch: Option<(usize, usize)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Required params
    for p in params.requireds().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(cop, source, &name, req.location(), outer_locals, block_cond_branch, diagnostics);
        }
    }

    // Optional params
    for p in params.optionals().iter() {
        if let Some(opt) = p.as_optional_parameter_node() {
            let name = std::str::from_utf8(opt.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(cop, source, &name, opt.location(), outer_locals, block_cond_branch, diagnostics);
        }
    }

    // Rest param
    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name_const) = rest_param.name() {
                let name = std::str::from_utf8(name_const.as_slice())
                    .unwrap_or("")
                    .to_string();
                check_shadow(cop, source, &name, rest_param.location(), outer_locals, block_cond_branch, diagnostics);
            }
        }
    }

    // Block param (&block)
    if let Some(block) = params.block() {
        if let Some(name_const) = block.name() {
            let name = std::str::from_utf8(name_const.as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(cop, source, &name, block.location(), outer_locals, block_cond_branch, diagnostics);
        }
    }
}

fn check_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    name: &str,
    loc: ruby_prism::Location<'_>,
    outer_locals: &HashMap<String, VarInfo>,
    block_cond_branch: Option<(usize, usize)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if name.is_empty() || name.starts_with('_') {
        return;
    }
    if let Some(info) = outer_locals.get(name) {
        // Skip if the outer variable and block are in different branches
        // of the same conditional (case/when, if/else).
        if is_different_conditional_branch(info.conditional_branch, block_cond_branch) {
            return;
        }
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Shadowing outer local variable - `{}`.", name),
        ));
    }
}

/// Collect names from MultiTargetNode entries in block parameters (destructuring).
/// E.g., `|(a, b, c)|` creates a MultiTargetNode with lefts [a, b, c].
fn collect_multi_target_names(node: &ruby_prism::MultiTargetNode<'_>, names: &mut HashSet<String>) {
    for target in node.lefts().iter() {
        if let Some(local) = target.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(local.name().as_slice()) {
                names.insert(s.to_string());
            }
        } else if let Some(inner) = target.as_multi_target_node() {
            collect_multi_target_names(&inner, names);
        }
    }
    if let Some(rest) = node.rest() {
        if let Some(splat) = rest.as_splat_node() {
            if let Some(expr) = splat.expression() {
                if let Some(local) = expr.as_required_parameter_node() {
                    if let Ok(s) = std::str::from_utf8(local.name().as_slice()) {
                        names.insert(s.to_string());
                    }
                }
            }
        }
    }
    for target in node.rights().iter() {
        if let Some(local) = target.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(local.name().as_slice()) {
                names.insert(s.to_string());
            }
        }
    }
}

/// Extract names from multi-target (destructured) block params and add to scope.
fn collect_multi_target_names_from_block_params(
    block_params: &ruby_prism::BlockParametersNode<'_>,
    scope: &mut HashMap<String, VarInfo>,
) {
    if let Some(params) = block_params.parameters() {
        for p in params.requireds().iter() {
            if let Some(mt) = p.as_multi_target_node() {
                let mut names = HashSet::new();
                collect_multi_target_names(&mt, &mut names);
                for name in names {
                    scope.insert(name, VarInfo { conditional_branch: None });
                }
            }
        }
    }
}

fn collect_param_names_into(params: &ruby_prism::ParametersNode<'_>, scope: &mut HashSet<String>) {
    for p in params.requireds().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(req.name().as_slice()) {
                scope.insert(s.to_string());
            }
        }
    }
    for p in params.optionals().iter() {
        if let Some(opt) = p.as_optional_parameter_node() {
            if let Ok(s) = std::str::from_utf8(opt.name().as_slice()) {
                scope.insert(s.to_string());
            }
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name) = rest_param.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    scope.insert(s.to_string());
                }
            }
        }
    }
    for p in params.keywords().iter() {
        if let Some(kw) = p.as_required_keyword_parameter_node() {
            if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                scope.insert(s.trim_end_matches(':').to_string());
            }
        } else if let Some(kw) = p.as_optional_keyword_parameter_node() {
            if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                scope.insert(s.trim_end_matches(':').to_string());
            }
        }
    }
    if let Some(block) = params.block() {
        if let Some(name) = block.name() {
            if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                scope.insert(s.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ShadowingOuterLocalVariable, "cops/lint/shadowing_outer_local_variable");
}
