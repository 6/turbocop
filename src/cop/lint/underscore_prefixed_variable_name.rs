use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct UnderscorePrefixedVariableName;

impl Cop for UnderscorePrefixedVariableName {
    fn name(&self) -> &'static str {
        "Lint/UnderscorePrefixedVariableName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut visitor = DefFinder {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct DefFinder<'a, 'src> {
    cop: &'a UnderscorePrefixedVariableName,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for DefFinder<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.check_def(node);
        // Don't recurse into nested defs
    }
}

impl DefFinder<'_, '_> {
    fn check_def(&mut self, def_node: &ruby_prism::DefNode<'_>) {
        // Collect underscore-prefixed param names with their offsets
        let mut underscore_vars: Vec<(String, usize)> = Vec::new();

        if let Some(params) = def_node.parameters() {
            collect_underscore_params(&params, &mut underscore_vars);
        }

        // Collect underscore-prefixed local variable writes in the body
        if let Some(body) = def_node.body() {
            let mut write_collector = WriteCollector {
                writes: Vec::new(),
            };
            write_collector.visit(&body);
            underscore_vars.extend(write_collector.writes);
        }

        if underscore_vars.is_empty() {
            return;
        }

        // Collect all local variable reads in the body
        let mut reads = HashSet::new();
        if let Some(body) = def_node.body() {
            let mut read_collector = ReadCollector {
                reads: &mut reads,
            };
            read_collector.visit(&body);
        }

        // Check for implicit forwarding (bare `super` or `binding`)
        let mut fwd_checker = ForwardingChecker {
            has_forwarding: false,
        };
        if let Some(body) = def_node.body() {
            fwd_checker.visit(&body);
        }

        for (name, offset) in &underscore_vars {
            // If there's bare super/binding and the var is NOT explicitly read,
            // don't flag it (it's implicitly forwarded)
            if fwd_checker.has_forwarding && !reads.contains(name.as_str()) {
                continue;
            }

            if reads.contains(name.as_str()) {
                let (line, col) = self.source.offset_to_line_col(*offset);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    col,
                    "Do not use prefix `_` for a variable that is used.".to_string(),
                ));
            }
        }
    }
}

fn collect_underscore_params(
    params: &ruby_prism::ParametersNode<'_>,
    out: &mut Vec<(String, usize)>,
) {
    for param in params.requireds().iter() {
        if let Some(req) = param.as_required_parameter_node() {
            let name = std::str::from_utf8(req.name().as_slice()).unwrap_or("");
            if name.starts_with('_') && name != "_" {
                out.push((name.to_string(), req.location().start_offset()));
            }
        }
    }

    for param in params.optionals().iter() {
        if let Some(opt) = param.as_optional_parameter_node() {
            let name = std::str::from_utf8(opt.name().as_slice()).unwrap_or("");
            if name.starts_with('_') && name != "_" {
                out.push((name.to_string(), opt.name_loc().start_offset()));
            }
        }
    }

    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name_const) = rest_param.name() {
                let name = std::str::from_utf8(name_const.as_slice()).unwrap_or("");
                if name.starts_with('_') && name != "_" {
                    if let Some(name_loc) = rest_param.name_loc() {
                        out.push((name.to_string(), name_loc.start_offset()));
                    }
                }
            }
        }
    }
}

/// Collects underscore-prefixed local variable writes.
struct WriteCollector {
    writes: Vec<(String, usize)>,
}

impl<'pr> Visit<'pr> for WriteCollector {
    fn visit_local_variable_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if name.starts_with('_') && name != "_" {
            self.writes
                .push((name.to_string(), node.name_loc().start_offset()));
        }
        // Visit the value expression
        self.visit(&node.value());
    }

    // Don't cross into nested defs/classes/modules
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

/// Collects all local variable reads.
struct ReadCollector<'a> {
    reads: &'a mut HashSet<String>,
}

impl<'pr> Visit<'pr> for ReadCollector<'_> {
    fn visit_local_variable_read_node(
        &mut self,
        node: &ruby_prism::LocalVariableReadNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        self.reads.insert(name.to_string());
    }

    // Don't cross into nested defs/classes/modules
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

/// Checks for bare `super` (ForwardingSuperNode) or `binding` calls without args.
struct ForwardingChecker {
    has_forwarding: bool,
}

impl<'pr> Visit<'pr> for ForwardingChecker {
    fn visit_forwarding_super_node(
        &mut self,
        _node: &ruby_prism::ForwardingSuperNode<'pr>,
    ) {
        self.has_forwarding = true;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"binding"
            && node.receiver().is_none()
            && node.arguments().is_none()
        {
            self.has_forwarding = true;
        }
        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UnderscorePrefixedVariableName,
        "cops/lint/underscore_prefixed_variable_name"
    );
}
