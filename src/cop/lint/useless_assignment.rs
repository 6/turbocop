use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct UselessAssignment;

impl Cop for UselessAssignment {
    fn name(&self) -> &'static str {
        "Lint/UselessAssignment"
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
        let mut visitor = UselessAssignVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct UselessAssignVisitor<'a, 'src> {
    cop: &'a UselessAssignment,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

struct WriteInfo {
    name: String,
    offset: usize,
}

struct WriteReadCollector {
    writes: Vec<WriteInfo>,
    reads: HashSet<String>,
    has_forwarding_super: bool,
}

impl WriteReadCollector {
    fn new() -> Self {
        Self {
            writes: Vec::new(),
            reads: HashSet::new(),
            has_forwarding_super: false,
        }
    }

    fn add_read(&mut self, name_bytes: &[u8]) {
        let name = std::str::from_utf8(name_bytes).unwrap_or("").to_string();
        self.reads.insert(name);
    }
}

impl<'pr> Visit<'pr> for WriteReadCollector {
    fn visit_local_variable_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.writes.push(WriteInfo {
            name,
            offset: node.name_loc().start_offset(),
        });
        // Visit the value (it might contain reads)
        self.visit(&node.value());
    }

    fn visit_local_variable_read_node(
        &mut self,
        node: &ruby_prism::LocalVariableReadNode<'pr>,
    ) {
        self.add_read(node.name().as_slice());
    }

    // Compound assignments (+=, -=, *=, etc.) are both reads AND writes.
    // `x += 1` means `x = x + 1`, so x is read before being written.
    // Any prior assignment to x is therefore used.
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.add_read(node.name().as_slice());
        self.visit(&node.value());
    }

    // `x ||= val` reads x first (to check if it's falsy), so prior assignment is used.
    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.add_read(node.name().as_slice());
        self.visit(&node.value());
    }

    // `x &&= val` reads x first (to check if it's truthy), so prior assignment is used.
    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.add_read(node.name().as_slice());
        self.visit(&node.value());
    }

    // Bare `super` (no args, no parens) implicitly forwards all method parameters,
    // so any assignment to a parameter variable is "used".
    fn visit_forwarding_super_node(
        &mut self,
        _node: &ruby_prism::ForwardingSuperNode<'pr>,
    ) {
        self.has_forwarding_super = true;
    }

    // Singleton method definitions (def var.method) use the variable as a receiver.
    // We must NOT recurse into the def body (it's a new scope), but we DO need to
    // check the receiver for local variable reads.
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            if let Some(read_node) = receiver.as_local_variable_read_node() {
                self.add_read(read_node.name().as_slice());
            }
        }
        // Don't recurse into the body — it's a new scope
    }

    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

/// Extract parameter names from a DefNode's parameter list.
fn collect_param_names(node: &ruby_prism::DefNode<'_>) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(params) = node.parameters() {
        for p in params.requireds().iter() {
            if let Some(req) = p.as_required_parameter_node() {
                if let Ok(s) = std::str::from_utf8(req.name().as_slice()) {
                    names.insert(s.to_string());
                }
            }
        }
        for p in params.optionals().iter() {
            if let Some(opt) = p.as_optional_parameter_node() {
                if let Ok(s) = std::str::from_utf8(opt.name().as_slice()) {
                    names.insert(s.to_string());
                }
            }
        }
        if let Some(rest) = params.rest() {
            if let Some(rest_param) = rest.as_rest_parameter_node() {
                if let Some(name) = rest_param.name() {
                    if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                        names.insert(s.to_string());
                    }
                }
            }
        }
        for p in params.keywords().iter() {
            if let Some(kw) = p.as_required_keyword_parameter_node() {
                if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                    names.insert(s.trim_end_matches(':').to_string());
                }
            } else if let Some(kw) = p.as_optional_keyword_parameter_node() {
                if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                    names.insert(s.trim_end_matches(':').to_string());
                }
            }
        }
        if let Some(kw_rest) = params.keyword_rest() {
            if let Some(kw_rest_param) = kw_rest.as_keyword_rest_parameter_node() {
                if let Some(name) = kw_rest_param.name() {
                    if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                        names.insert(s.to_string());
                    }
                }
            }
        }
        if let Some(block) = params.block() {
            if let Some(name) = block.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    names.insert(s.to_string());
                }
            }
        }
    }
    names
}

impl<'pr> Visit<'pr> for UselessAssignVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(body) = node.body() {
            let mut collector = WriteReadCollector::new();
            collector.visit(&body);

            // If bare `super` is used, all method parameters are implicitly read
            if collector.has_forwarding_super {
                let param_names = collect_param_names(node);
                for name in &param_names {
                    collector.reads.insert(name.clone());
                }
            }

            // Find writes that are never read
            for write in &collector.writes {
                if write.name.starts_with('_') {
                    continue;
                }
                if !collector.reads.contains(&write.name) {
                    let (line, column) = self.source.offset_to_line_col(write.offset);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        format!("Useless assignment to variable - `{}`.", write.name),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessAssignment, "cops/lint/useless_assignment");
}
