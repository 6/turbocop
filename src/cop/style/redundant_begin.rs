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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
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

impl RedundantBeginVisitor<'_> {
    /// Check if a body (block, lambda, etc.) contains a redundant `begin` block.
    /// A `begin..rescue..end` or `begin..ensure..end` inside a block body is
    /// redundant when it's the only statement, because the block itself supports
    /// rescue/ensure directly.
    fn check_body_begin(&mut self, body: Option<ruby_prism::Node<'_>>) {
        let body = match body {
            Some(b) => b,
            None => return,
        };

        // The body is either a StatementsNode containing a single BeginNode,
        // or directly a BeginNode
        let begin_node = if let Some(b) = body.as_begin_node() {
            b
        } else if let Some(stmts) = body.as_statements_node() {
            let body_nodes: Vec<_> = stmts.body().into_iter().collect();
            if body_nodes.len() != 1 {
                // Multiple statements — visit children and return
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
                // No explicit begin — visit children
                if let Some(stmts) = begin_node.statements() {
                    for child in stmts.body().iter() {
                        self.visit(&child);
                    }
                }
                if let Some(rescue) = begin_node.rescue_clause() {
                    self.visit_rescue_node(&rescue);
                }
                if let Some(ensure) = begin_node.ensure_clause() {
                    self.visit_ensure_node(&ensure);
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

        // Visit children for nested checks
        if let Some(stmts) = begin_node.statements() {
            for child in stmts.body().iter() {
                self.visit(&child);
            }
        }
        if let Some(rescue) = begin_node.rescue_clause() {
            self.visit_rescue_node(&rescue);
        }
        if let Some(ensure) = begin_node.ensure_clause() {
            self.visit_ensure_node(&ensure);
        }
    }

    /// Check if an assignment value is a redundant `begin` block.
    /// `x = begin...end` or `x ||= begin...end` is redundant when:
    /// - The begin has an explicit `begin` keyword
    /// - There is only a single statement in the body
    /// - There are no rescue/ensure/else clauses
    fn check_assignment_begin(&mut self, value: &ruby_prism::Node<'_>) {
        let begin_node = match value.as_begin_node() {
            Some(b) => b,
            None => {
                // Continue visiting for nested structures
                self.visit(value);
                return;
            }
        };

        // Must have an explicit `begin` keyword
        let begin_kw_loc = match begin_node.begin_keyword_loc() {
            Some(loc) => loc,
            None => {
                self.visit(value);
                return;
            }
        };

        // If it has rescue/ensure/else, the begin is NOT redundant
        if begin_node.rescue_clause().is_some()
            || begin_node.ensure_clause().is_some()
            || begin_node.else_clause().is_some()
        {
            // Visit children for nested checks
            if let Some(stmts) = begin_node.statements() {
                for child in stmts.body().iter() {
                    self.visit(&child);
                }
            }
            if let Some(rescue) = begin_node.rescue_clause() {
                self.visit_rescue_node(&rescue);
            }
            if let Some(ensure) = begin_node.ensure_clause() {
                self.visit_ensure_node(&ensure);
            }
            return;
        }

        // Must have exactly one statement in the body
        let stmts = match begin_node.statements() {
            Some(s) => s,
            None => return,
        };
        let body_nodes: Vec<_> = stmts.body().into_iter().collect();
        if body_nodes.len() != 1 {
            // Multiple statements — begin is not redundant in assignment context
            for child in body_nodes.iter() {
                self.visit(child);
            }
            return;
        }

        let offset = begin_kw_loc.start_offset();
        let (line, column) = self.source.offset_to_line_col(offset);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Redundant `begin` block detected.".to_string(),
        ));

        // Visit the begin body for nested checks
        for child in body_nodes.iter() {
            self.visit(child);
        }
    }
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

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Skip brace blocks ({ }) — only do..end blocks support implicit
        // begin/rescue. Ruby syntax doesn't allow bare rescue in { } blocks.
        if node.opening_loc().as_slice() == b"{" {
            // Still need to visit children for nested checks
            if let Some(body) = node.body() {
                self.visit(&body);
            }
            return;
        }
        self.check_body_begin(node.body());
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // RuboCop skips stabby lambdas entirely — they don't support implicit
        // begin/rescue in their body (even with do..end form).
        // Still visit children for nested checks.
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_instance_variable_or_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_class_variable_or_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_global_variable_or_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_or_write_node(&mut self, node: &ruby_prism::ConstantOrWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantBegin, "cops/style/redundant_begin");
}
