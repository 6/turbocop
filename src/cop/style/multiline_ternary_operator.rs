use std::collections::HashSet;

use crate::cop::shared::method_identifier_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for multi-line ternary operator expressions.
///
/// ## Investigation (2026-03-10)
///
/// Root cause of 5 FPs: RuboCop's `offense?` method includes a `node.source != replacement(node)`
/// check. When the ternary's parent is `return`, `break`, `next`, `send`, or `csend` (but not an
/// assignment method send), RuboCop uses a single-line ternary as the replacement. If the ternary
/// source already equals that single-line reconstruction (i.e., only the condition is multiline
/// due to method chaining across lines), then `source == replacement` and no offense is registered.
///
/// Example of skipped pattern (parent is a method call):
/// ```ruby
/// do_something(arg
///                .foo ? bar : baz)
/// ```
/// The condition `arg\n.foo` spans lines, making the ternary "multiline", but the `?`, `:`, and
/// branches are on one line. The single-line replacement would be identical, so RuboCop skips it.
///
/// Fix: Switched from `check_node` to `check_source` with a custom visitor to track parent
/// context. When the parent is return/break/next/send/csend (not assignment method), we compute
/// the single-line ternary replacement and compare it against the source. If they match, we skip.
pub struct MultilineTernaryOperator;

struct TernaryVisitor<'a> {
    source: &'a SourceFile,
    cop_name: &'static str,
    diagnostics: Vec<Diagnostic>,
    /// Offsets of ternary IfNodes that were already checked by a parent-aware visitor method.
    handled: HashSet<usize>,
}

impl TernaryVisitor<'_> {
    fn check_ternary(
        &mut self,
        if_node: &ruby_prism::IfNode<'_>,
        single_line_enforced: bool,
    ) -> bool {
        // Must be a ternary (no if_keyword_loc)
        if if_node.if_keyword_loc().is_some() {
            return false;
        }

        // Must be multiline
        let loc = if_node.location();
        let (start_line, _) = self.source.offset_to_line_col(loc.start_offset());
        let (end_line, _) = self
            .source
            .offset_to_line_col(loc.end_offset().saturating_sub(1));

        if start_line == end_line {
            return false;
        }

        // RuboCop's `source != replacement` check:
        // When parent enforces single-line ternary, the replacement is
        // "#{cond} ? #{if_branch} : #{else_branch}". If that equals the source, skip.
        if single_line_enforced && self.source_equals_single_line_replacement(if_node) {
            return false;
        }
        // For non-single-line-enforced parent, the replacement is always an if/else block
        // which never equals a ternary source, so we always flag.

        let message = if single_line_enforced {
            "Avoid multi-line ternary operators, use single-line instead."
        } else {
            "Avoid multi-line ternary operators, use `if` or `unless` instead."
        };

        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(Diagnostic {
            path: self.source.path_str().to_string(),
            location: Location { line, column },
            severity: Severity::Convention,
            cop_name: self.cop_name.to_string(),
            message: message.to_string(),
            corrected: false,
        });
        true
    }

    fn source_equals_single_line_replacement(&self, if_node: &ruby_prism::IfNode<'_>) -> bool {
        let loc = if_node.location();
        let node_source = self
            .source
            .byte_slice(loc.start_offset(), loc.end_offset(), "");

        let predicate = if_node.predicate();
        let statements = match if_node.statements() {
            Some(s) => s,
            None => return false,
        };
        let subsequent = match if_node.subsequent() {
            Some(s) => s,
            None => return false,
        };

        let cond_src = self.source.byte_slice(
            predicate.location().start_offset(),
            predicate.location().end_offset(),
            "",
        );

        let if_branch_src = {
            let body: Vec<_> = statements.body().iter().collect();
            if body.len() == 1 {
                self.source.byte_slice(
                    body[0].location().start_offset(),
                    body[0].location().end_offset(),
                    "",
                )
            } else {
                return false;
            }
        };

        let else_branch_src = if let Some(else_node) = subsequent.as_else_node() {
            if let Some(else_stmts) = else_node.statements() {
                let body: Vec<_> = else_stmts.body().iter().collect();
                if body.len() == 1 {
                    self.source.byte_slice(
                        body[0].location().start_offset(),
                        body[0].location().end_offset(),
                        "",
                    )
                } else {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        };

        let replacement = format!("{cond_src} ? {if_branch_src} : {else_branch_src}");
        node_source == replacement
    }

    /// Check if a call node is an assignment method (e.g., `a.foo=`).
    fn is_assignment_method_call(call_node: &ruby_prism::CallNode<'_>) -> bool {
        method_identifier_predicates::is_assignment_method(call_node.name().as_slice())
    }
}

impl<'pr> Visit<'pr> for TernaryVisitor<'_> {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                if let Some(if_node) = arg.as_if_node() {
                    self.handled.insert(if_node.location().start_offset());
                    self.check_ternary(&if_node, true);
                }
            }
        }
        ruby_prism::visit_return_node(self, node);
    }

    fn visit_break_node(&mut self, node: &ruby_prism::BreakNode<'pr>) {
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                if let Some(if_node) = arg.as_if_node() {
                    self.handled.insert(if_node.location().start_offset());
                    self.check_ternary(&if_node, true);
                }
            }
        }
        ruby_prism::visit_break_node(self, node);
    }

    fn visit_next_node(&mut self, node: &ruby_prism::NextNode<'pr>) {
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                if let Some(if_node) = arg.as_if_node() {
                    self.handled.insert(if_node.location().start_offset());
                    self.check_ternary(&if_node, true);
                }
            }
        }
        ruby_prism::visit_next_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if !Self::is_assignment_method_call(node) {
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    if let Some(if_node) = arg.as_if_node() {
                        self.handled.insert(if_node.location().start_offset());
                        self.check_ternary(&if_node, true);
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // If this IfNode was already handled by a parent-aware visitor method, skip.
        if !self.handled.contains(&node.location().start_offset()) {
            self.check_ternary(node, false);
        }
        ruby_prism::visit_if_node(self, node);
    }
}

impl Cop for MultilineTernaryOperator {
    fn name(&self) -> &'static str {
        "Style/MultilineTernaryOperator"
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
        let mut visitor = TernaryVisitor {
            source,
            cop_name: self.name(),
            diagnostics: Vec::new(),
            handled: HashSet::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MultilineTernaryOperator,
        "cops/style/multiline_ternary_operator"
    );
}
