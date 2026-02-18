use std::collections::HashSet;

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
    ) -> Vec<Diagnostic> {
        let mut visitor = ShadowVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            scopes: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct ShadowVisitor<'a, 'src> {
    cop: &'a ShadowingOuterLocalVariable,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Stack of sets of local variable names in scope.
    /// Each set represents the variables visible at a particular scope level.
    scopes: Vec<HashSet<String>>,
}

impl ShadowVisitor<'_, '_> {
    fn current_locals(&self) -> HashSet<String> {
        let mut all = HashSet::new();
        for scope in &self.scopes {
            all.extend(scope.iter().cloned());
        }
        all
    }

    fn add_local(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string());
        }
    }
}

impl<'pr> Visit<'pr> for ShadowVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // def creates a new scope — push a fresh scope with param names
        let mut scope = HashSet::new();
        if let Some(params) = node.parameters() {
            collect_param_names_into(&params, &mut scope);
        }
        self.scopes.push(scope);
        ruby_prism::visit_def_node(self, node);
        self.scopes.pop();
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // class body is a new scope
        self.scopes.push(HashSet::new());
        ruby_prism::visit_class_node(self, node);
        self.scopes.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.scopes.push(HashSet::new());
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
            self.scopes.push(HashSet::new());
            ruby_prism::visit_block_node(self, node);
            self.scopes.pop();
            return;
        }

        let outer_locals = self.current_locals();

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
                        &mut self.diagnostics,
                    );
                }

                // Check block-local variables (|a; b| — b is a block-local)
                for local in block_params.locals().iter() {
                    let name = std::str::from_utf8(local.as_block_local_variable_node().map_or(&[][..], |n| n.name().as_slice()))
                        .unwrap_or("")
                        .to_string();
                    if !name.is_empty() && !name.starts_with('_') && outer_locals.contains(&name) {
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

        // Push a new scope for the block body. Do NOT merge back into the
        // outer scope — RuboCop's VariableForce treats block-internal
        // variables as local to the block, not visible to sibling blocks.
        self.scopes.push(HashSet::new());
        ruby_prism::visit_block_node(self, node);
        self.scopes.pop();
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambdas behave like blocks for shadowing purposes
        let outer_locals = self.current_locals();

        if let Some(params_node) = node.parameters() {
            if let Some(block_params) = params_node.as_block_parameters_node() {
                if let Some(inner_params) = block_params.parameters() {
                    check_block_params_shadow(
                        self.cop,
                        self.source,
                        &inner_params,
                        &outer_locals,
                        &mut self.diagnostics,
                    );
                }
            }
        }

        // Lambda creates an isolated scope — do NOT merge back.
        self.scopes.push(HashSet::new());
        ruby_prism::visit_lambda_node(self, node);
        self.scopes.pop();
    }

    // Handle top-level assignments (outside any method)
    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        self.scopes.push(HashSet::new());
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

fn check_block_params_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    params: &ruby_prism::ParametersNode<'_>,
    outer_locals: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Required params
    for p in params.requireds().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(cop, source, &name, req.location(), outer_locals, diagnostics);
        }
    }

    // Optional params
    for p in params.optionals().iter() {
        if let Some(opt) = p.as_optional_parameter_node() {
            let name = std::str::from_utf8(opt.name().as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(cop, source, &name, opt.location(), outer_locals, diagnostics);
        }
    }

    // Rest param
    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name_const) = rest_param.name() {
                let name = std::str::from_utf8(name_const.as_slice())
                    .unwrap_or("")
                    .to_string();
                check_shadow(cop, source, &name, rest_param.location(), outer_locals, diagnostics);
            }
        }
    }

    // Block param (&block)
    if let Some(block) = params.block() {
        if let Some(name_const) = block.name() {
            let name = std::str::from_utf8(name_const.as_slice())
                .unwrap_or("")
                .to_string();
            check_shadow(cop, source, &name, block.location(), outer_locals, diagnostics);
        }
    }
}

fn check_shadow(
    cop: &ShadowingOuterLocalVariable,
    source: &SourceFile,
    name: &str,
    loc: ruby_prism::Location<'_>,
    outer_locals: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if name.is_empty() || name.starts_with('_') {
        return;
    }
    if outer_locals.contains(name) {
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Shadowing outer local variable - `{}`.", name),
        ));
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
