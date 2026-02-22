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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let min_body_length = config.get_usize("MinBodyLength", 1);
        let _allow_consecutive = config.get_bool("AllowConsecutiveConditionals", false);
        let max_line_length = config.get_usize("MaxLineLength", 120);
        let mut visitor = GuardClauseVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            min_body_length,
            max_line_length,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct GuardClauseVisitor<'a, 'src> {
    cop: &'a GuardClause,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    min_body_length: usize,
    max_line_length: usize,
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

        // Skip if condition spans multiple lines
        let predicate = node.predicate();
        if self.is_multiline(&predicate) {
            return;
        }

        // Skip if condition assigns a local variable used in the if body
        if let Some(body_stmts) = node.statements() {
            for stmt in body_stmts.body().iter() {
                if self.assigned_lvar_used_in_branch(&predicate, &stmt) {
                    return;
                }
            }
        }

        // Check min body length
        let end_offset = node
            .end_keyword_loc()
            .map(|l| l.start_offset())
            .unwrap_or(node.location().end_offset());
        if !self.meets_min_body_length(if_keyword_loc.start_offset(), end_offset) {
            return;
        }

        let condition_src = self.node_source(&predicate);
        let example = format!("return unless {}", condition_src);
        let (line, column) = self
            .source
            .offset_to_line_col(if_keyword_loc.start_offset());

        // Skip if guard clause would be too long and body is trivial
        if self.too_long_and_trivial(
            column,
            &example,
            node.statements(),
            node.subsequent().is_some(),
        ) {
            return;
        }

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

        // Skip if condition spans multiple lines
        let predicate = node.predicate();
        if self.is_multiline(&predicate) {
            return;
        }

        // Skip if condition assigns a local variable used in the body
        if let Some(body_stmts) = node.statements() {
            for stmt in body_stmts.body().iter() {
                if self.assigned_lvar_used_in_branch(&predicate, &stmt) {
                    return;
                }
            }
        }

        // Check min body length
        let end_offset = node
            .end_keyword_loc()
            .map(|l| l.start_offset())
            .unwrap_or(node.location().end_offset());
        if !self.meets_min_body_length(keyword_loc.start_offset(), end_offset) {
            return;
        }

        let condition_src = self.node_source(&predicate);
        let example = format!("return if {}", condition_src);
        let (line, column) = self.source.offset_to_line_col(keyword_loc.start_offset());

        // Skip if guard clause would be too long and body is trivial
        if self.too_long_and_trivial(
            column,
            &example,
            node.statements(),
            node.else_clause().is_some(),
        ) {
            return;
        }

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

    /// Check if a node spans multiple lines.
    fn is_multiline(&self, node: &ruby_prism::Node<'_>) -> bool {
        let loc = node.location();
        let (start_line, _) = self.source.offset_to_line_col(loc.start_offset());
        let (end_line, _) = self.source.offset_to_line_col(loc.end_offset());
        end_line > start_line
    }

    /// Check if the condition contains local variable assignments that are used
    /// in the if body. RuboCop skips guard clause suggestions in this case because
    /// the assignment is meaningful -- the assigned value is used in the body.
    fn assigned_lvar_used_in_branch(
        &self,
        condition: &ruby_prism::Node<'_>,
        body: &ruby_prism::Node<'_>,
    ) -> bool {
        let assigned_names = collect_lvar_write_names(condition);
        if assigned_names.is_empty() {
            return false;
        }
        let used_names = collect_lvar_read_names(body);
        assigned_names.iter().any(|name| used_names.contains(name))
    }

    /// Check if the guard clause would exceed max line length AND the body is trivial.
    /// "Trivial" means a single-branch if/unless with a body that is not itself an
    /// if/unless or begin block. In this case, RuboCop skips the offense.
    fn too_long_and_trivial(
        &self,
        column: usize,
        example: &str,
        statements: Option<ruby_prism::StatementsNode<'_>>,
        has_else: bool,
    ) -> bool {
        let total_len = column + example.len();
        if total_len <= self.max_line_length {
            return false;
        }
        // Too long -- check if body is trivial
        if has_else {
            return false;
        }
        let stmts = match statements {
            Some(s) => s,
            None => return true, // empty body is trivial
        };
        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return false;
        }
        let single = &body_nodes[0];
        // Not trivial if the body is itself an if/unless or begin
        if single.as_if_node().is_some()
            || single.as_unless_node().is_some()
            || single.as_begin_node().is_some()
        {
            return false;
        }
        true
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

/// Visitor to collect local variable write names from a node tree.
struct LvarWriteCollector {
    names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for LvarWriteCollector {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.names.push(node.name().as_slice().to_vec());
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        // Multi-assignment targets: (var, obj = ...)
        self.names.push(node.name().as_slice().to_vec());
    }
}

/// Visitor to collect local variable read names from a node tree.
struct LvarReadCollector {
    names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for LvarReadCollector {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        self.names.push(node.name().as_slice().to_vec());
    }
}

fn collect_lvar_write_names(node: &ruby_prism::Node<'_>) -> Vec<Vec<u8>> {
    let mut collector = LvarWriteCollector { names: Vec::new() };
    collector.visit(node);
    collector.names
}

fn collect_lvar_read_names(node: &ruby_prism::Node<'_>) -> Vec<Vec<u8>> {
    let mut collector = LvarReadCollector { names: Vec::new() };
    collector.visit(node);
    collector.names
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
