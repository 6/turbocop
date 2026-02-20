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
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut visitor = UselessAssignVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            inside_def: false,
            inside_analyzed_block: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
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

/// Scope-aware write/read collector for block-level analysis.
///
/// Unlike the flat `WriteReadCollector` (used for def bodies), this tracks
/// writes and reads per-scope in a tree. Each nested block creates a child
/// scope. A write is useless only if the variable is not read in the same
/// scope, any ancestor scope, or any descendant scope — but NOT sibling
/// scopes. This correctly handles sibling `it` blocks in RSpec where each
/// block is an independent closure.
struct ScopeData {
    parent: Option<usize>,
    children: Vec<usize>,
    writes: Vec<WriteInfo>,
    reads: HashSet<String>,
    has_binding_call: bool,
}

struct ScopedCollector {
    scopes: Vec<ScopeData>,
    scope_stack: Vec<usize>,
}

impl ScopedCollector {
    fn new() -> Self {
        let root = ScopeData {
            parent: None,
            children: Vec::new(),
            writes: Vec::new(),
            reads: HashSet::new(),
            has_binding_call: false,
        };
        Self {
            scopes: vec![root],
            scope_stack: vec![0],
        }
    }

    fn current_scope(&self) -> usize {
        *self.scope_stack.last().unwrap()
    }

    fn enter_scope(&mut self) {
        let parent = self.current_scope();
        let idx = self.scopes.len();
        self.scopes.push(ScopeData {
            parent: Some(parent),
            children: Vec::new(),
            writes: Vec::new(),
            reads: HashSet::new(),
            has_binding_call: false,
        });
        self.scopes[parent].children.push(idx);
        self.scope_stack.push(idx);
    }

    fn leave_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn add_write(&mut self, name: String, offset: usize) {
        let idx = self.current_scope();
        self.scopes[idx].writes.push(WriteInfo { name, offset });
    }

    fn add_read(&mut self, name_bytes: &[u8]) {
        let name = std::str::from_utf8(name_bytes).unwrap_or("").to_string();
        let idx = self.current_scope();
        self.scopes[idx].reads.insert(name);
    }

    /// Find the "home scope" for a variable — the highest ancestor that also
    /// has a write for this variable name. In Ruby, blocks share the enclosing
    /// scope, so a variable first declared in an outer scope is accessible to
    /// all nested blocks (including siblings). The home scope determines where
    /// the variable "lives."
    fn find_home_scope(&self, scope_idx: usize, name: &str) -> usize {
        let mut home = scope_idx;
        let mut current = self.scopes[scope_idx].parent;
        while let Some(idx) = current {
            if self.scopes[idx].writes.iter().any(|w| w.name == name) {
                home = idx;
            }
            current = self.scopes[idx].parent;
        }
        home
    }

    /// Check if a variable name is read anywhere in the subtree rooted at
    /// the given scope (the scope itself and all descendants).
    fn is_read_in_subtree(&self, scope_idx: usize, name: &str) -> bool {
        if self.scopes[scope_idx].reads.contains(name) {
            return true;
        }
        for &child_idx in &self.scopes[scope_idx].children {
            if self.is_read_in_subtree(child_idx, name) {
                return true;
            }
        }
        false
    }

    /// Check if any scope in the entire tree has a binding call.
    fn has_any_binding(&self) -> bool {
        self.scopes.iter().any(|s| s.has_binding_call)
    }
}

impl<'pr> Visit<'pr> for ScopedCollector {
    fn visit_local_variable_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.add_write(name, node.name_loc().start_offset());
        self.visit(&node.value());
    }

    fn visit_local_variable_read_node(
        &mut self,
        node: &ruby_prism::LocalVariableReadNode<'pr>,
    ) {
        self.add_read(node.name().as_slice());
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.add_read(node.name().as_slice());
        self.visit(&node.value());
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.add_read(node.name().as_slice());
        self.visit(&node.value());
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.add_read(node.name().as_slice());
        self.visit(&node.value());
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.receiver().is_none()
            && node.name().as_slice() == b"binding"
            && (node.arguments().is_none()
                || node.arguments().map_or(true, |a| a.arguments().is_empty()))
        {
            let idx = self.current_scope();
            self.scopes[idx].has_binding_call = true;
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.enter_scope();
        ruby_prism::visit_block_node(self, node);
        self.leave_scope();
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.enter_scope();
        ruby_prism::visit_lambda_node(self, node);
        self.leave_scope();
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            if let Some(read_node) = receiver.as_local_variable_read_node() {
                self.add_read(read_node.name().as_slice());
            }
        }
        // Don't recurse into the body — it's a new hard scope
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

    /// Analyze a scoped collector for useless assignments.
    ///
    /// For each write, finds the variable's "home scope" — the highest
    /// ancestor that also writes to the same variable. In Ruby, blocks share
    /// the enclosing scope, so a variable initialized in an outer scope is
    /// accessible to ALL nested blocks (including siblings). A write is
    /// useless only if the variable is not read anywhere in the home scope's
    /// subtree (the home scope itself + all its descendants).
    ///
    /// This correctly handles:
    /// - Sibling `it` blocks: each block assigns locally (no parent write),
    ///   so home scope = the block itself. Sibling reads don't count.
    /// - Sequential blocks sharing a parent variable: `x = nil; block { x = 1 };
    ///   block { use(x) }` — home scope = parent (which has the `x = nil`
    ///   write). The sibling block's read IS in the parent's subtree.
    fn analyze_scoped(&mut self, collector: &ScopedCollector) {
        if collector.has_any_binding() {
            return;
        }
        for (scope_idx, scope) in collector.scopes.iter().enumerate() {
            for write in &scope.writes {
                if write.name.starts_with('_') {
                    continue;
                }
                // Find where the variable "lives" — the highest ancestor
                // that also writes to this variable name.
                let home = collector.find_home_scope(scope_idx, &write.name);
                // Check if the variable is read anywhere in the home scope's
                // subtree (home + all descendants, including sibling blocks).
                if collector.is_read_in_subtree(home, &write.name) {
                    continue;
                }
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
        // def's analysis.
        if !self.inside_def && !self.inside_analyzed_block {
            if let Some(body) = node.body() {
                let mut collector = ScopedCollector::new();
                // Block parameters are implicitly "read" (they're arguments, not
                // useless assignments). Add them to the root scope's reads.
                if let Some(params) = node.parameters() {
                    let root = collector.current_scope();
                    collect_block_param_names(&params, &mut collector.scopes[root].reads);
                }
                collector.visit(&body);
                self.analyze_scoped(&collector);
            }

            // Mark that we're inside an analyzed block so nested blocks don't
            // re-analyze (they're already covered by this block's scoped
            // collector which tracks nested scopes).
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
                let mut collector = ScopedCollector::new();
                if let Some(params) = node.parameters() {
                    if let Some(params) = params.as_block_parameters_node() {
                        if let Some(inner) = params.parameters() {
                            let root = collector.current_scope();
                            collect_parameters_node_names(&inner, &mut collector.scopes[root].reads);
                        }
                    }
                }
                collector.visit(&body);
                self.analyze_scoped(&collector);
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
