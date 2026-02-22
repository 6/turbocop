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
        let _ignore_implicit = config.get_bool("IgnoreImplicitReferences", false);
        let mut visitor = ShadowedArgVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ShadowedArgVisitor<'a, 'src> {
    cop: &'a ShadowedArgument,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
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
    names
}

impl ShadowedArgVisitor<'_, '_> {
    fn check_body_with_params(
        &mut self,
        param_names: &[Vec<u8>],
        body: Option<ruby_prism::Node<'_>>,
    ) {
        if param_names.is_empty() {
            return;
        }
        let body = match body {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };
        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.is_empty() {
            return;
        }
        for param_name in param_names {
            self.check_one_param(param_name, &body_nodes);
        }
    }

    fn check_one_param(&mut self, param_name: &[u8], body_nodes: &[ruby_prism::Node<'_>]) {
        for stmt in body_nodes {
            if let Some(write) = stmt.as_local_variable_write_node() {
                let write_name = write.name().as_slice();
                if write_name != param_name {
                    if node_references_local(stmt, param_name) {
                        return;
                    }
                    continue;
                }
                // RHS references the param => not shadowing (e.g. foo = foo.strip)
                if node_references_local(&write.value(), param_name) {
                    return;
                }
                let loc = write.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Argument `{}` was shadowed by a local variable before it was used.",
                        String::from_utf8_lossy(write_name)
                    ),
                ));
                return;
            } else if let Some(op_write) = stmt.as_local_variable_operator_write_node() {
                if op_write.name().as_slice() == param_name {
                    return;
                }
            } else if let Some(or_write) = stmt.as_local_variable_or_write_node() {
                if or_write.name().as_slice() == param_name {
                    return;
                }
            } else if let Some(and_write) = stmt.as_local_variable_and_write_node() {
                if and_write.name().as_slice() == param_name {
                    return;
                }
            }
            if node_references_local(stmt, param_name) {
                return;
            }
            if is_conditional_node(stmt) {
                return;
            }
        }
    }
}

/// Check if a node tree contains a local variable read of the given name.
fn node_references_local(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    let mut finder = LocalRefFinder {
        name: name.to_vec(),
        found: false,
    };
    finder.visit(node);
    finder.found
}

struct LocalRefFinder {
    name: Vec<u8>,
    found: bool,
}

impl<'pr> Visit<'pr> for LocalRefFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.name.as_slice() {
            self.found = true;
        }
    }

    fn visit_forwarding_super_node(&mut self, _node: &ruby_prism::ForwardingSuperNode<'pr>) {
        self.found = true;
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

fn is_conditional_node(node: &ruby_prism::Node<'_>) -> bool {
    node.as_if_node().is_some()
        || node.as_unless_node().is_some()
        || node.as_case_node().is_some()
        || node.as_case_match_node().is_some()
        || node.as_begin_node().is_some()
        || node.as_rescue_node().is_some()
        || node.as_while_node().is_some()
        || node.as_until_node().is_some()
}

impl<'pr> Visit<'pr> for ShadowedArgVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(params) = node.parameters() {
            let names = collect_param_names(&params);
            self.check_body_with_params(&names, node.body());
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        if let Some(params_node) = node.parameters() {
            if let Some(bp) = params_node.as_block_parameters_node() {
                if let Some(inner) = bp.parameters() {
                    let names = collect_param_names(&inner);
                    self.check_body_with_params(&names, node.body());
                }
            }
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if let Some(params_node) = node.parameters() {
            if let Some(bp) = params_node.as_block_parameters_node() {
                if let Some(inner) = bp.parameters() {
                    let names = collect_param_names(&inner);
                    self.check_body_with_params(&names, node.body());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ShadowedArgument, "cops/lint/shadowed_argument");
}
