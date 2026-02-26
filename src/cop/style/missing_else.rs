use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MissingElse;

impl Cop for MissingElse {
    fn name(&self) -> &'static str {
        "Style/MissingElse"
    }

    fn default_enabled(&self) -> bool {
        false // Matches vendor config/default.yml: Enabled: false
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
        let style = config.get_str("EnforcedStyle", "both");
        let mut visitor = MissingElseVisitor {
            cop: self,
            source,
            style,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MissingElseVisitor<'a> {
    cop: &'a MissingElse,
    source: &'a SourceFile,
    style: &'a str,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for MissingElseVisitor<'_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        if self.style == "if" || self.style == "both" {
            // Check if this is a regular block if (not unless, not ternary, not modifier)
            // Modifier if (e.g. `return x if cond`) has no end keyword
            if let Some(kw_loc) = node.if_keyword_loc() {
                let kw = kw_loc.as_slice();
                if kw == b"if" && node.end_keyword_loc().is_some() {
                    // RuboCop's IfNode#else? returns true when there's an elsif,
                    // so if/elsif/end is NOT flagged. Only flag when there's no
                    // subsequent clause at all (plain if/end).
                    if node.subsequent().is_none() {
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "`if` condition requires an `else`-clause.".to_string(),
                        ));
                    }
                }
            }
        }

        // Visit children
        self.visit(&node.predicate());
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
        if let Some(sub) = node.subsequent() {
            self.visit(&sub);
        }
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        if self.style == "case" || self.style == "both" {
            // Check if there's an else clause
            if node.else_clause().is_none() {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "`case` condition requires an `else`-clause.".to_string(),
                ));
            }
        }

        // Visit children
        if let Some(pred) = node.predicate() {
            self.visit(&pred);
        }
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit(&else_clause.as_node());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MissingElse, "cops/style/missing_else");
}
