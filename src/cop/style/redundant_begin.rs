use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantBegin;

impl Cop for RedundantBegin {
    fn name(&self) -> &'static str {
        "Style/RedundantBegin"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut visitor = RedundantBeginVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RedundantBeginVisitor<'a> {
    cop: &'a RedundantBegin,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for RedundantBeginVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let body = match node.body() {
            Some(b) => b,
            None => return,
        };

        // The body might be a BeginNode directly or a StatementsNode containing
        // a single BeginNode
        let begin_node = if let Some(b) = body.as_begin_node() {
            b
        } else if let Some(stmts) = body.as_statements_node() {
            let body_nodes: Vec<_> = stmts.body().into_iter().collect();
            if body_nodes.len() != 1 {
                // Continue visiting children for nested defs/begins
                for child in body_nodes.iter() {
                    self.visit(child);
                }
                return;
            }
            match body_nodes[0].as_begin_node() {
                Some(b) => b,
                None => {
                    self.visit(&body_nodes[0]);
                    return;
                }
            }
        } else {
            self.visit(&body);
            return;
        };

        // Must have an explicit `begin` keyword
        let begin_kw_loc = match begin_node.begin_keyword_loc() {
            Some(loc) => loc,
            None => {
                // Visit the begin body for nested checks
                if let Some(stmts) = begin_node.statements() {
                    for child in stmts.body().iter() {
                        self.visit(&child);
                    }
                }
                return;
            }
        };

        let offset = begin_kw_loc.start_offset();
        let (line, column) = self.source.offset_to_line_col(offset);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Redundant `begin` block detected.".to_string(),
        ));

        // Visit the begin body for nested checks
        if let Some(stmts) = begin_node.statements() {
            for child in stmts.body().iter() {
                self.visit(&child);
            }
        }
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        // Continue visiting children to find nested begin nodes (e.g. nested defs)
        if let Some(stmts) = node.statements() {
            for child in stmts.body().iter() {
                self.visit(&child);
            }
        }
        if let Some(rescue) = node.rescue_clause() {
            self.visit_rescue_node(&rescue);
        }
        if let Some(ensure) = node.ensure_clause() {
            self.visit_ensure_node(&ensure);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantBegin, "cops/style/redundant_begin");
}
