use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Matches `!= nil` while preserving RuboCop's predicate-method exception:
/// only the final predicate expression itself is exempt, not nested checks
/// inside a larger `&&`/`rescue` expression, while parenthesized final checks
/// are still exempt.
///
/// When a predicate method body is wrapped in begin/rescue or begin/ensure,
/// the inner `!= nil` expression is NOT exempt. RuboCop ignores the entire
/// rescue-wrapped block (not its children), so inner comparisons are still
/// flagged. Fixed by not unwrapping BeginNode with rescue/ensure clauses in
/// `last_predicate_expression`.
pub struct NonNilCheck;

impl Cop for NonNilCheck {
    fn name(&self) -> &'static str {
        "Style/NonNilCheck"
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
        let include_semantic_changes = config.get_bool("IncludeSemanticChanges", false);
        let mut visitor = NonNilCheckVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            include_semantic_changes,
            in_predicate_method: false,
            predicate_last_expr_span: None,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct NonNilCheckVisitor<'a, 'src> {
    cop: &'a NonNilCheck,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    include_semantic_changes: bool,
    in_predicate_method: bool,
    /// Span of the final predicate expression after unwrapping transparent wrappers.
    predicate_last_expr_span: Option<NodeSpan>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct NodeSpan {
    start: usize,
    end: usize,
}

impl NodeSpan {
    fn from_node(node: &ruby_prism::Node<'_>) -> Self {
        let loc = node.location();
        Self {
            start: loc.start_offset(),
            end: loc.end_offset(),
        }
    }
}

fn last_predicate_expression<'pr>(node: ruby_prism::Node<'pr>) -> Option<ruby_prism::Node<'pr>> {
    if let Some(statements) = node.as_statements_node() {
        statements
            .body()
            .iter()
            .last()
            .and_then(last_predicate_expression)
    } else if let Some(begin) = node.as_begin_node() {
        // When a begin block has rescue/ensure clauses, RuboCop ignores the
        // entire block (not its inner statements), so inner != nil calls are
        // still flagged.  Only unwrap plain begin blocks without rescue/ensure.
        if begin.rescue_clause().is_some() || begin.ensure_clause().is_some() {
            return Some(node);
        }
        begin
            .statements()
            .and_then(|statements| statements.body().iter().last())
            .and_then(last_predicate_expression)
    } else if let Some(parentheses) = node.as_parentheses_node() {
        parentheses.body().and_then(last_predicate_expression)
    } else {
        Some(node)
    }
}

impl<'pr> Visit<'pr> for NonNilCheckVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let name = node.name().as_slice();
        let is_predicate = name.ends_with(b"?");

        let prev_in_predicate = self.in_predicate_method;
        let prev_last_expr_span = self.predicate_last_expr_span;

        self.in_predicate_method = is_predicate;
        self.predicate_last_expr_span = if is_predicate {
            node.body()
                .and_then(last_predicate_expression)
                .map(|node| NodeSpan::from_node(&node))
        } else {
            None
        };

        ruby_prism::visit_def_node(self, node);

        self.in_predicate_method = prev_in_predicate;
        self.predicate_last_expr_span = prev_last_expr_span;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method = node.name().as_slice();

        // Pattern 1: x != nil
        if method == b"!=" {
            if let Some(args) = node.arguments() {
                let args_vec: Vec<_> = args.arguments().iter().collect();
                if args_vec.len() == 1
                    && args_vec[0].as_nil_node().is_some()
                    && node.receiver().is_some()
                {
                    // RuboCop skips the last expression of predicate methods (def foo?)
                    let is_predicate_return = self.in_predicate_method
                        && self.predicate_last_expr_span
                            == Some(NodeSpan::from_node(&node.as_node()));
                    if !is_predicate_return {
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        if self.include_semantic_changes {
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Explicit non-nil checks are usually redundant.".to_string(),
                            ));
                        } else {
                            let receiver_src =
                                std::str::from_utf8(node.receiver().unwrap().location().as_slice())
                                    .unwrap_or("x");
                            let current_src = std::str::from_utf8(loc.as_slice()).unwrap_or("");
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                format!("Prefer `!{}.nil?` over `{}`.", receiver_src, current_src),
                            ));
                        }
                    }
                }
            }
        }

        // Pattern 2: !x.nil? (only with IncludeSemanticChanges)
        if self.include_semantic_changes && method == b"!" {
            if let Some(receiver) = node.receiver() {
                if let Some(inner_call) = receiver.as_call_node() {
                    if inner_call.name().as_slice() == b"nil?"
                        && inner_call.arguments().is_none()
                        && inner_call.receiver().is_some()
                    {
                        let is_predicate_return = self.in_predicate_method
                            && self.predicate_last_expr_span
                                == Some(NodeSpan::from_node(&node.as_node()));
                        if !is_predicate_return {
                            let loc = node.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Explicit non-nil checks are usually redundant.".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NonNilCheck, "cops/style/non_nil_check");
}
