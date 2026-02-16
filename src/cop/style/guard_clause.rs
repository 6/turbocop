use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct GuardClause;

impl Cop for GuardClause {
    fn name(&self) -> &'static str {
        "Style/GuardClause"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let min_body_length = config.get_usize("MinBodyLength", 1);
        let _allow_consecutive = config.get_bool("AllowConsecutiveConditionals", false);
        let mut visitor = GuardClauseVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            min_body_length,
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct GuardClauseVisitor<'a, 'src> {
    cop: &'a GuardClause,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    min_body_length: usize,
}

impl GuardClauseVisitor<'_, '_> {
    /// Check if the ending of a method body is an if/unless that could be a guard clause.
    fn check_ending_body(&mut self, body: &ruby_prism::Node<'_>) {
        if let Some(if_node) = body.as_if_node() {
            self.check_ending_if_node(&if_node);
        } else if let Some(unless_node) = body.as_unless_node() {
            self.check_ending_unless_node(&unless_node);
        } else if let Some(stmts) = body.as_statements_node() {
            // Body is a StatementsNode (begin block) - check last statement
            let body_nodes: Vec<_> = stmts.body().iter().collect();
            if let Some(last) = body_nodes.last() {
                if let Some(if_node) = last.as_if_node() {
                    self.check_ending_if_node(&if_node);
                } else if let Some(unless_node) = last.as_unless_node() {
                    self.check_ending_unless_node(&unless_node);
                }
            }
        }
    }

    fn check_ending_if_node(&mut self, node: &ruby_prism::IfNode<'_>) {
        // if_keyword_loc() is None for ternary
        let if_keyword_loc = match node.if_keyword_loc() {
            Some(loc) => loc,
            None => return, // ternary
        };

        // Check that the keyword is actually "if" (not elsif)
        if if_keyword_loc.as_slice() != b"if" {
            return;
        }

        // Modifier if: the node location starts before the keyword (at the body expression)
        if node.location().start_offset() != if_keyword_loc.start_offset() {
            return;
        }

        // If it has a subsequent branch (else/elsif), skip for ending guard clause check
        if node.subsequent().is_some() {
            return;
        }

        // Check min body length
        let end_offset = node
            .end_keyword_loc()
            .map(|l| l.start_offset())
            .unwrap_or(node.location().end_offset());
        if !self.meets_min_body_length(if_keyword_loc.start_offset(), end_offset) {
            return;
        }

        let condition_src = self.node_source(&node.predicate());
        let example = format!("return unless {}", condition_src);
        let (line, column) = self.source.offset_to_line_col(if_keyword_loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!(
                "Use a guard clause (`{}`) instead of wrapping the code inside a conditional expression.",
                example
            ),
        ));
    }

    fn check_ending_unless_node(&mut self, node: &ruby_prism::UnlessNode<'_>) {
        // Check for modifier form: in modifier unless, the node location starts
        // before the keyword (at the expression). If the node start != keyword start,
        // it's a modifier form.
        let keyword_loc = node.keyword_loc();
        if node.location().start_offset() != keyword_loc.start_offset() {
            return;
        }

        // If it has an else branch, skip
        if node.else_clause().is_some() {
            return;
        }

        // Check min body length
        let end_offset = node
            .end_keyword_loc()
            .map(|l| l.start_offset())
            .unwrap_or(node.location().end_offset());
        if !self.meets_min_body_length(keyword_loc.start_offset(), end_offset) {
            return;
        }

        let condition_src = self.node_source(&node.predicate());
        let example = format!("return if {}", condition_src);
        let (line, column) = self.source.offset_to_line_col(keyword_loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!(
                "Use a guard clause (`{}`) instead of wrapping the code inside a conditional expression.",
                example
            ),
        ));
    }

    fn meets_min_body_length(&self, start_offset: usize, end_offset: usize) -> bool {
        let (start_line, _) = self.source.offset_to_line_col(start_offset);
        let (end_line, _) = self.source.offset_to_line_col(end_offset);
        let body_lines = if end_line > start_line + 1 {
            end_line - start_line - 1
        } else if end_line > start_line {
            0
        } else {
            1
        };
        body_lines >= self.min_body_length
    }

    fn node_source(&self, node: &ruby_prism::Node<'_>) -> String {
        let loc = node.location();
        let bytes = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
        String::from_utf8_lossy(bytes).to_string()
    }
}

impl<'pr> Visit<'pr> for GuardClauseVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(body) = node.body() {
            self.check_ending_body(&body);
        }
        ruby_prism::visit_def_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(GuardClause, "cops/style/guard_clause");
}
