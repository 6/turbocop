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
            inside_def: false,
            inside_analyzed_block: false,
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct UselessAssignVisitor<'a, 'src> {
    cop: &'a UselessAssignment,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when inside a def node. Block analysis is skipped since the def
    /// analysis already covers nested blocks.
    inside_def: bool,
    /// True when inside a block that's being independently analyzed. Prevents
    /// nested blocks from being double-analyzed.
    inside_analyzed_block: bool,
}

struct WriteInfo {
    name: String,
    offset: usize,
}

struct WriteReadCollector {
    writes: Vec<WriteInfo>,
    reads: HashSet<String>,
    has_forwarding_super: bool,
    has_binding_call: bool,
}

impl WriteReadCollector {
    fn new() -> Self {
        Self {
            writes: Vec::new(),
            reads: HashSet::new(),
            has_forwarding_super: false,
            has_binding_call: false,
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

    // A bare `binding` call (no receiver, no arguments) captures the entire
    // local scope, so all local variable assignments are implicitly "used".
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.receiver().is_none()
            && node.name().as_slice() == b"binding"
            && (node.arguments().is_none()
                || node.arguments().map_or(true, |a| a.arguments().is_empty()))
        {
            self.has_binding_call = true;
        }
        // Continue visiting children (arguments, blocks, etc.)
        ruby_prism::visit_call_node(self, node);
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

impl UselessAssignVisitor<'_, '_> {
    /// Analyze a body (from a def or block) for useless assignments and report diagnostics.
    fn analyze_body(&mut self, collector: &WriteReadCollector) {
        // A bare `binding` call captures all local variables, so every
        // assignment is implicitly used.
        if collector.has_binding_call {
            return;
        }
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

            self.analyze_body(&collector);
        }

        // Mark that we're inside a def so nested blocks don't re-analyze
        let was_inside_def = self.inside_def;
        self.inside_def = true;
        ruby_prism::visit_def_node(self, node);
        self.inside_def = was_inside_def;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Only analyze blocks that are NOT inside a def AND not already inside
        // a block being analyzed. Blocks inside defs are already covered by the
        // def's analysis. Nested blocks share scope with the outermost analyzed
        // block, so they shouldn't be analyzed independently.
        if !self.inside_def && !self.inside_analyzed_block {
            if let Some(body) = node.body() {
                let mut collector = WriteReadCollector::new();
                collector.visit(&body);

                // Block parameters are implicitly "read" (they're arguments, not
                // useless assignments). Add them to reads so they're not flagged.
                if let Some(params) = node.parameters() {
                    collect_block_param_names(&params, &mut collector.reads);
                }

                self.analyze_body(&collector);
            }

            // Mark that we're inside an analyzed block so nested blocks don't
            // re-analyze (they're already covered by this block's collector
            // which recurses into them).
            let was_inside = self.inside_analyzed_block;
            self.inside_analyzed_block = true;
            ruby_prism::visit_block_node(self, node);
            self.inside_analyzed_block = was_inside;
        } else {
            // Continue visiting to find nested defs
            ruby_prism::visit_block_node(self, node);
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambdas create scopes similar to blocks
        if !self.inside_def && !self.inside_analyzed_block {
            if let Some(body) = node.body() {
                let mut collector = WriteReadCollector::new();
                collector.visit(&body);

                if let Some(params) = node.parameters() {
                    if let Some(params) = params.as_block_parameters_node() {
                        if let Some(inner) = params.parameters() {
                            collect_parameters_node_names(&inner, &mut collector.reads);
                        }
                    }
                }

                self.analyze_body(&collector);
            }

            let was_inside = self.inside_analyzed_block;
            self.inside_analyzed_block = true;
            ruby_prism::visit_lambda_node(self, node);
            self.inside_analyzed_block = was_inside;
        } else {
            ruby_prism::visit_lambda_node(self, node);
        }
    }
}

/// Collect parameter names from a BlockNode's parameter list into a reads set.
fn collect_block_param_names(params: &ruby_prism::Node<'_>, reads: &mut HashSet<String>) {
    if let Some(block_params) = params.as_block_parameters_node() {
        if let Some(inner_params) = block_params.parameters() {
            collect_parameters_node_names(&inner_params, reads);
        }
    } else if let Some(numbered) = params.as_numbered_parameters_node() {
        // Numbered params (_1, _2, etc.) — add them as reads
        for i in 1..=numbered.maximum() {
            reads.insert(format!("_{i}"));
        }
    }
}

/// Collect parameter names from a ParametersNode into a reads set.
fn collect_parameters_node_names(params: &ruby_prism::ParametersNode<'_>, reads: &mut HashSet<String>) {
    for p in params.requireds().iter() {
        if let Some(req) = p.as_required_parameter_node() {
            if let Ok(s) = std::str::from_utf8(req.name().as_slice()) {
                reads.insert(s.to_string());
            }
        }
    }
    for p in params.optionals().iter() {
        if let Some(opt) = p.as_optional_parameter_node() {
            if let Ok(s) = std::str::from_utf8(opt.name().as_slice()) {
                reads.insert(s.to_string());
            }
        }
    }
    if let Some(rest) = params.rest() {
        if let Some(rest_param) = rest.as_rest_parameter_node() {
            if let Some(name) = rest_param.name() {
                if let Ok(s) = std::str::from_utf8(name.as_slice()) {
                    reads.insert(s.to_string());
                }
            }
        }
    }
    for p in params.keywords().iter() {
        if let Some(kw) = p.as_required_keyword_parameter_node() {
            if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                reads.insert(s.trim_end_matches(':').to_string());
            }
        } else if let Some(kw) = p.as_optional_keyword_parameter_node() {
            if let Ok(s) = std::str::from_utf8(kw.name().as_slice()) {
                reads.insert(s.trim_end_matches(':').to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessAssignment, "cops/lint/useless_assignment");
}
