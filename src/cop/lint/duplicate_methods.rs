use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct DuplicateMethods;

impl Cop for DuplicateMethods {
    fn name(&self) -> &'static str {
        "Lint/DuplicateMethods"
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
        let mut visitor = DupMethodVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct DupMethodVisitor<'a, 'src> {
    cop: &'a DuplicateMethods,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

/// Build a method key that distinguishes instance methods from singleton methods.
fn def_method_key(def_node: &ruby_prism::DefNode<'_>) -> Vec<u8> {
    let name = def_node.name().as_slice();
    if def_node.receiver().is_some() {
        let mut key = b"self.".to_vec();
        key.extend_from_slice(name);
        key
    } else {
        name.to_vec()
    }
}

fn check_body_for_dup_methods(
    cop: &DuplicateMethods,
    source: &SourceFile,
    body: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let stmts = if let Some(stmts) = body.as_statements_node() {
        stmts
    } else {
        return;
    };

    let mut seen: HashMap<Vec<u8>, usize> = HashMap::new();

    for stmt in stmts.body().iter() {
        if let Some(def_node) = stmt.as_def_node() {
            let key = def_method_key(&def_node);
            let loc = def_node.location();
            if seen.contains_key(&key) {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Duplicated method definition.".to_string(),
                ));
            } else {
                seen.insert(key, loc.start_offset());
            }
        }
        // Per RuboCop: alias counts as defining a method
        if let Some(alias_node) = stmt.as_alias_method_node() {
            if let Some(name_sym) = alias_node.new_name().as_symbol_node() {
                // Self-alias (alias foo foo) is allowed per RuboCop
                if let Some(orig_sym) = alias_node.old_name().as_symbol_node() {
                    let new_bytes = name_sym.unescaped();
                    let old_bytes = orig_sym.unescaped();
                    if new_bytes == old_bytes {
                        continue;
                    }
                }
                let key = name_sym.unescaped().to_vec();
                let loc = alias_node.location();
                if seen.contains_key(&key) {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(cop.diagnostic(
                        source,
                        line,
                        column,
                        "Duplicated method definition.".to_string(),
                    ));
                } else {
                    seen.insert(key, loc.start_offset());
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for DupMethodVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            check_body_for_dup_methods(self.cop, self.source, &body, &mut self.diagnostics);
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            check_body_for_dup_methods(self.cop, self.source, &body, &mut self.diagnostics);
        }
        ruby_prism::visit_module_node(self, node);
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        if let Some(body) = node.body() {
            check_body_for_dup_methods(self.cop, self.source, &body, &mut self.diagnostics);
        }
        ruby_prism::visit_singleton_class_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateMethods, "cops/lint/duplicate_methods");
}
