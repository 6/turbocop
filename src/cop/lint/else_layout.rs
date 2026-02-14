use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ElseLayout;

impl Cop for ElseLayout {
    fn name(&self) -> &'static str {
        "Lint/ElseLayout"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Must be a keyword if/unless (not ternary)
        if if_node.if_keyword_loc().is_none() {
            return Vec::new();
        }

        // Check the subsequent (else/elsif) clause
        let subsequent = match if_node.subsequent() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // We only care about else clauses, not elsif
        // An else clause in Prism is represented as an ElseNode
        let else_node = match subsequent.as_else_node() {
            Some(e) => e,
            None => return Vec::new(),
        };

        let else_kw_loc = else_node.else_keyword_loc();
        let (else_line, _) = source.offset_to_line_col(else_kw_loc.start_offset());

        // Check if there's a statement on the same line as else
        let statements = match else_node.statements() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body = statements.body();
        let first_stmt = match body.first() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let first_loc = first_stmt.location();
        let (stmt_line, stmt_col) = source.offset_to_line_col(first_loc.start_offset());

        if stmt_line == else_line {
            return vec![self.diagnostic(
                source,
                stmt_line,
                stmt_col,
                "Odd `else` layout detected. Code on the same line as `else` is not allowed.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ElseLayout, "cops/lint/else_layout");
}
