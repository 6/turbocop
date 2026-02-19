use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantAssignment;

impl Cop for RedundantAssignment {
    fn name(&self) -> &'static str {
        "Style/RedundantAssignment"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut visitor = RedundantAssignmentVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RedundantAssignmentVisitor<'a> {
    cop: &'a RedundantAssignment,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl RedundantAssignmentVisitor<'_> {
    fn check_body(&mut self, stmts: &[ruby_prism::Node<'_>]) {
        if stmts.len() < 2 {
            return;
        }

        let last = &stmts[stmts.len() - 1];
        let second_last = &stmts[stmts.len() - 2];

        // Check if the last statement is a local variable read
        let var_name = if let Some(lvar) = last.as_local_variable_read_node() {
            lvar.name().as_slice().to_vec()
        } else {
            return;
        };

        // Check if the second-to-last is an assignment to the same variable
        if let Some(write) = second_last.as_local_variable_write_node() {
            if write.name().as_slice() == var_name {
                let loc = second_last.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Redundant assignment before returning detected.".to_string(),
                ));
            }
        }
    }
}

impl<'pr> Visit<'pr> for RedundantAssignmentVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let body_stmts: Vec<_> = stmts.body().iter().collect();
                self.check_body(&body_stmts);
            }
            // Also check for begin/rescue blocks
            if let Some(begin) = body.as_begin_node() {
                if let Some(stmts) = begin.statements() {
                    let body_stmts: Vec<_> = stmts.body().iter().collect();
                    self.check_body(&body_stmts);
                }
                if let Some(rescue) = begin.rescue_clause() {
                    self.check_rescue(&rescue);
                }
            }
            self.visit(&body);
        }
    }
}

impl RedundantAssignmentVisitor<'_> {
    fn check_rescue(&mut self, rescue: &ruby_prism::RescueNode<'_>) {
        if let Some(stmts) = rescue.statements() {
            let body_stmts: Vec<_> = stmts.body().iter().collect();
            self.check_body(&body_stmts);
        }
        if let Some(subsequent) = rescue.subsequent() {
            self.check_rescue(&subsequent);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantAssignment, "cops/style/redundant_assignment");
}
