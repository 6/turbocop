use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/EmptyCaseCondition flags `case` statements with no predicate expression
/// and suggests using `if`/`elsif` chains instead.
///
/// ## Investigation findings
///
/// - RuboCop only suppresses this cop when the empty `case` node's direct
///   parent is `return`, `break`, `next`, `send`, or `csend`.
/// - nitrocop previously modeled that with sticky visitor state, which leaked
///   through intermediate nodes such as `ArgumentsNode`, `KeywordHashNode`, and
///   `AssocNode`.
/// - That leaked suppression produced false negatives for hash values like
///   `result.merge(key => case ... end)`, where the direct parent is an
///   association node and RuboCop still registers an offense.
///
/// ## Fix
///
/// Track the full traversal stack and inspect only the direct parent of each
/// `CaseNode`, while keeping the existing branch-return suppression.
pub struct EmptyCaseCondition;

impl Cop for EmptyCaseCondition {
    fn name(&self) -> &'static str {
        "Style/EmptyCaseCondition"
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
        let mut visitor = EmptyCaseVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            node_kind_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ParentKind {
    /// return, break, next, send, csend — case not supported as if-replacement
    Unsupported,
    /// Any other parent type
    Other,
}

struct EmptyCaseVisitor<'a> {
    cop: &'a EmptyCaseCondition,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    node_kind_stack: Vec<ParentKind>,
}

/// Visitor that checks if a subtree contains any return node.
struct ReturnFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for ReturnFinder {
    fn visit_return_node(&mut self, _node: &ruby_prism::ReturnNode<'pr>) {
        self.found = true;
    }
}

/// Check if any branch body of a case node contains a return statement.
fn branch_contains_return(case_node: &ruby_prism::CaseNode<'_>) -> bool {
    let mut finder = ReturnFinder { found: false };
    for when_ref in case_node.conditions().iter() {
        if let Some(when_node) = when_ref.as_when_node() {
            if let Some(stmts) = when_node.statements() {
                finder.visit(&stmts.as_node());
                if finder.found {
                    return true;
                }
            }
        }
    }
    if let Some(else_clause) = case_node.else_clause() {
        if let Some(stmts) = else_clause.statements() {
            finder.visit(&stmts.as_node());
            if finder.found {
                return true;
            }
        }
    }
    false
}

fn node_parent_kind(node: &ruby_prism::Node<'_>) -> ParentKind {
    match node {
        ruby_prism::Node::ReturnNode { .. }
        | ruby_prism::Node::BreakNode { .. }
        | ruby_prism::Node::NextNode { .. }
        | ruby_prism::Node::CallNode { .. } => ParentKind::Unsupported,
        _ => ParentKind::Other,
    }
}

impl EmptyCaseVisitor<'_> {
    fn direct_parent_kind(&self) -> ParentKind {
        self.node_kind_stack
            .iter()
            .rev()
            .nth(1)
            .copied()
            .unwrap_or(ParentKind::Other)
    }
}

impl<'pr> Visit<'pr> for EmptyCaseVisitor<'_> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.node_kind_stack.push(node_parent_kind(&node));
    }

    fn visit_branch_node_leave(&mut self) {
        self.node_kind_stack.pop();
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.node_kind_stack.push(node_parent_kind(&node));
    }

    fn visit_leaf_node_leave(&mut self) {
        self.node_kind_stack.pop();
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        // Only flag if case has no predicate (empty case condition)
        if node.predicate().is_none() && self.direct_parent_kind() != ParentKind::Unsupported {
            // Skip if any branch body contains a return statement (or descendant)
            if !branch_contains_return(node) {
                let case_kw_loc = node.case_keyword_loc();
                let case_offset = case_kw_loc.start_offset();
                let (line, column) = self.source.offset_to_line_col(case_offset);
                self.diagnostics.push(
                    self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Do not use empty `case` condition, instead use an `if` expression."
                            .to_string(),
                    ),
                );
            }
        }

        ruby_prism::visit_case_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyCaseCondition, "cops/style/empty_case_condition");
}
