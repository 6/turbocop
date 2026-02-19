use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct UselessAccessModifier;

impl Cop for UselessAccessModifier {
    fn name(&self) -> &'static str {
        "Lint/UselessAccessModifier"
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
    ) {
        let _context_creating = config.get_string_array("ContextCreatingMethods");
        let method_creating = config.get_string_array("MethodCreatingMethods").unwrap_or_default();
        let mut visitor = UselessAccessVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            method_creating_methods: method_creating,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessKind {
    Public,
    Private,
    Protected,
}

impl AccessKind {
    fn as_str(self) -> &'static str {
        match self {
            AccessKind::Public => "public",
            AccessKind::Private => "private",
            AccessKind::Protected => "protected",
        }
    }
}

fn get_access_modifier(call: &ruby_prism::CallNode<'_>) -> Option<AccessKind> {
    if call.receiver().is_some() || call.arguments().is_some() {
        return None;
    }
    let name = call.name().as_slice();
    match name {
        b"public" => Some(AccessKind::Public),
        b"private" => Some(AccessKind::Private),
        b"protected" => Some(AccessKind::Protected),
        _ => None,
    }
}

fn is_method_definition(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(def_node) = node.as_def_node() {
        // Singleton methods (def self.foo) are NOT affected by access modifiers,
        // so they don't count as method definitions for our purposes.
        if def_node.receiver().is_none() {
            return true;
        }
        return false;
    }
    // attr_reader/writer/accessor
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() {
            let name = call.name().as_slice();
            if name == b"attr_reader"
                || name == b"attr_writer"
                || name == b"attr_accessor"
                || name == b"attr"
                || name == b"define_method"
            {
                return true;
            }
        }
    }
    false
}

/// Check if a node is a call to one of the configured MethodCreatingMethods.
fn is_method_creating_call(node: &ruby_prism::Node<'_>, method_creating_methods: &[String]) -> bool {
    if method_creating_methods.is_empty() {
        return false;
    }
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() {
            let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
            return method_creating_methods.iter().any(|m| m == name);
        }
    }
    false
}

fn has_method_definition_in_subtree(node: &ruby_prism::Node<'_>) -> bool {
    if is_method_definition(node) {
        return true;
    }
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for stmt in stmts.body().iter() {
                if has_method_definition_in_subtree(&stmt) {
                    return true;
                }
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            if has_method_definition_in_subtree(&subsequent) {
                return true;
            }
        }
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            for stmt in stmts.body().iter() {
                if has_method_definition_in_subtree(&stmt) {
                    return true;
                }
            }
        }
    }
    false
}

fn check_scope(
    cop: &UselessAccessModifier,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    stmts: &ruby_prism::StatementsNode<'_>,
    method_creating_methods: &[String],
) {
    let body: Vec<_> = stmts.body().iter().collect();

    let mut current_vis = AccessKind::Public;
    let mut unused_modifier: Option<(usize, AccessKind)> = None;

    for stmt in &body {
        if let Some(call) = stmt.as_call_node() {
            if let Some(modifier_kind) = get_access_modifier(&call) {
                if modifier_kind == current_vis {
                    // Repeated modifier - always useless
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(cop.diagnostic(
                        source,
                        line,
                        column,
                        format!("Useless `{}` access modifier.", current_vis.as_str()),
                    ));
                } else {
                    // New modifier - flag previous if unused
                    if let Some((offset, old_vis)) = unused_modifier {
                        let (line, column) = source.offset_to_line_col(offset);
                        diagnostics.push(cop.diagnostic(
                            source,
                            line,
                            column,
                            format!("Useless `{}` access modifier.", old_vis.as_str()),
                        ));
                    }
                    current_vis = modifier_kind;
                    unused_modifier = Some((call.location().start_offset(), modifier_kind));
                }
                continue;
            }
        }

        if has_method_definition_in_subtree(stmt) || is_method_creating_call(stmt, method_creating_methods) {
            unused_modifier = None;
        }
    }

    // If the last modifier was never followed by a method definition
    if let Some((offset, vis)) = unused_modifier {
        let (line, column) = source.offset_to_line_col(offset);
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Useless `{}` access modifier.", vis.as_str()),
        ));
    }
}

struct UselessAccessVisitor<'a, 'src> {
    cop: &'a UselessAccessModifier,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    method_creating_methods: Vec<String>,
}

impl<'pr> Visit<'pr> for UselessAccessVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                check_scope(self.cop, self.source, &mut self.diagnostics, &stmts, &self.method_creating_methods);
            }
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                check_scope(self.cop, self.source, &mut self.diagnostics, &stmts, &self.method_creating_methods);
            }
        }
        ruby_prism::visit_module_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessAccessModifier, "cops/lint/useless_access_modifier");
}
