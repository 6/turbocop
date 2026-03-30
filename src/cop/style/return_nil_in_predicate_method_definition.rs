use ruby_prism::Visit;
use std::path::{Component, Path};

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-30):
/// - FNs were concentrated in predicate methods whose implicit final value was `nil`
///   instead of an explicit `return nil`, including trailing `nil`, ternaries, and
///   final `if`/`unless` branches that evaluate to `nil`.
/// - The previous implementation only walked explicit `return` nodes, so it missed
///   RuboCop's narrower implicit-return check.
/// - Fixed by checking only the method body's implicit return position and recursing
///   through final `if`/`unless` branches, while still ignoring non-final `nil`
///   expressions in the middle of a method.
/// - Sampled corpus validation also surfaced two false positives under
///   `toys/.lib/...`: RuboCop skips hidden-path files during repo scans, but
///   nitrocop still fed them to this cop. As a stopgap within the cop's allowed
///   scope, skip files whose path contains a hidden component.
pub struct ReturnNilInPredicateMethodDefinition;

impl Cop for ReturnNilInPredicateMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/ReturnNilInPredicateMethodDefinition"
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
        if path_has_hidden_component(&source.path) {
            return;
        }

        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .unwrap_or_default();
        let allowed_patterns = config
            .get_string_array("AllowedPatterns")
            .unwrap_or_default();

        let mut visitor = PredicateReturnVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            allowed_methods,
            allowed_patterns,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct PredicateReturnVisitor<'a> {
    cop: &'a ReturnNilInPredicateMethodDefinition,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    allowed_methods: Vec<String>,
    allowed_patterns: Vec<String>,
}

impl<'pr> Visit<'pr> for PredicateReturnVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let name = node.name().as_slice();
        // Only check predicate methods (ending with ?)
        if !name.ends_with(b"?") {
            ruby_prism::visit_def_node(self, node);
            return;
        }

        // Check AllowedMethods
        let name_str = std::str::from_utf8(name).unwrap_or("");
        if self.allowed_methods.iter().any(|m| m == name_str) {
            return;
        }

        // Check AllowedPatterns
        for pattern in &self.allowed_patterns {
            if name_str.contains(pattern.as_str()) {
                return;
            }
        }

        // Scan body for return/return nil statements
        if let Some(body) = node.body() {
            let mut finder = ReturnFinder {
                returns: Vec::new(),
            };
            finder.visit(&body);

            for ret_loc in finder.returns {
                self.push_diagnostic(
                    ret_loc.0,
                    "Avoid using `return nil` or `return` in predicate methods.",
                );
            }

            self.handle_implicit_return_values(&body);
        }

        // Don't recurse into nested defs
    }
}

impl PredicateReturnVisitor<'_> {
    fn push_diagnostic(&mut self, offset: usize, message: &str) {
        let (line, column) = self.source.offset_to_line_col(offset);
        self.diagnostics.push(
            self.cop
                .diagnostic(self.source, line, column, message.to_string()),
        );
    }

    fn handle_implicit_return_values<'pr>(&mut self, node: &ruby_prism::Node<'pr>) {
        let Some(last_node) = last_implicit_return_node(node) else {
            return;
        };

        if let Some(if_node) = last_node.as_if_node() {
            self.handle_if_node(&if_node);
            return;
        }

        if let Some(unless_node) = last_node.as_unless_node() {
            self.handle_unless_node(&unless_node);
            return;
        }

        if let Some(nil_node) = last_node.as_nil_node() {
            self.push_diagnostic(
                nil_node.location().start_offset(),
                "Return `false` instead of `nil` in predicate methods.",
            );
        }
    }

    fn handle_if_node<'pr>(&mut self, node: &ruby_prism::IfNode<'pr>) {
        if let Some(statements) = node.statements() {
            self.handle_implicit_return_values(&statements.as_node());
        }

        if let Some(subsequent) = node.subsequent() {
            self.handle_implicit_return_values(&subsequent);
        }
    }

    fn handle_unless_node<'pr>(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        if let Some(statements) = node.statements() {
            self.handle_implicit_return_values(&statements.as_node());
        }

        if let Some(else_clause) = node.else_clause() {
            self.handle_implicit_return_values(&else_clause.as_node());
        }
    }
}

fn last_implicit_return_node<'pr>(node: &ruby_prism::Node<'pr>) -> Option<ruby_prism::Node<'pr>> {
    if let Some(if_node) = node.as_if_node() {
        return Some(if_node.as_node());
    }

    if let Some(unless_node) = node.as_unless_node() {
        return Some(unless_node.as_node());
    }

    if let Some(nil_node) = node.as_nil_node() {
        return Some(nil_node.as_node());
    }

    if let Some(statements) = node.as_statements_node() {
        return statements
            .body()
            .last()
            .and_then(|last| last_implicit_return_node(&last));
    }

    if let Some(else_node) = node.as_else_node() {
        return else_node
            .statements()
            .and_then(|statements| statements.body().last())
            .and_then(|last| last_implicit_return_node(&last));
    }

    if let Some(begin_node) = node.as_begin_node() {
        if begin_node.rescue_clause().is_none()
            && begin_node.else_clause().is_none()
            && begin_node.ensure_clause().is_none()
        {
            return begin_node
                .statements()
                .and_then(|statements| statements.body().last())
                .and_then(|last| last_implicit_return_node(&last));
        }

        return None;
    }

    if let Some(parentheses_node) = node.as_parentheses_node() {
        return parentheses_node
            .body()
            .and_then(|body| last_implicit_return_node(&body));
    }

    None
}

fn path_has_hidden_component(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::Normal(name)
                if name.to_str().is_some_and(|s| s.starts_with('.') && s != "." && s != "..")
        )
    })
}

struct ReturnFinder {
    returns: Vec<(usize, usize)>, // (start_offset, end_offset)
}

impl<'pr> Visit<'pr> for ReturnFinder {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        // Check for `return` (bare) or `return nil`
        let is_bare = node.arguments().is_none();
        let is_nil = if let Some(args) = node.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            arg_list.len() == 1 && arg_list[0].as_nil_node().is_some()
        } else {
            false
        };

        if is_bare || is_nil {
            self.returns
                .push((node.location().start_offset(), node.location().end_offset()));
        }
    }

    // Don't recurse into nested defs
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ReturnNilInPredicateMethodDefinition,
        "cops/style/return_nil_in_predicate_method_definition"
    );
}
