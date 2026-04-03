use crate::cop::shared::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

const MSG: &str = "Odd `else` layout detected. Did you mean to use `elsif`?";

/// Mirrors RuboCop's `Lint/ElseLayout`, including Prism's separate `UnlessNode`
/// handling so multiline `unless ... else expr` branches on the `else` line are
/// flagged the same way as `if ... else expr`.
pub struct ElseLayout;

impl Cop for ElseLayout {
    fn name(&self) -> &'static str {
        "Lint/ElseLayout"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(if_node) = node.as_if_node() {
            let if_kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };

            let else_node = if_node
                .subsequent()
                .and_then(|subsequent| subsequent.as_else_node());
            check_else_layout(
                self,
                source,
                if_kw_loc.start_offset(),
                if_node.location().end_offset(),
                if_node.then_keyword_loc().is_some(),
                else_node,
                diagnostics,
            );
            return;
        }

        if let Some(unless_node) = node.as_unless_node() {
            check_else_layout(
                self,
                source,
                unless_node.keyword_loc().start_offset(),
                unless_node.location().end_offset(),
                unless_node.then_keyword_loc().is_some(),
                unless_node.else_clause(),
                diagnostics,
            );
        }
    }
}

fn check_else_layout(
    cop: &ElseLayout,
    source: &SourceFile,
    conditional_start: usize,
    conditional_end: usize,
    has_then_keyword: bool,
    else_node: Option<ruby_prism::ElseNode<'_>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let else_node = match else_node {
        Some(node) => node,
        None => return,
    };

    // If the entire conditional is on a single line, skip (handled by Style/OneLineConditional).
    let (start_line, _) = source.offset_to_line_col(conditional_start);
    let end_offset = conditional_end.saturating_sub(1);
    let (end_line, _) = source.offset_to_line_col(end_offset);
    if start_line == end_line {
        return;
    }

    let statements = match else_node.statements() {
        Some(statements) => statements,
        None => return,
    };
    let body = statements.body();
    let first_stmt = match body.first() {
        Some(statement) => statement,
        None => return,
    };

    // RuboCop allows `if x then y \n else z \n end` and the equivalent `unless`
    // form when the else body is a single statement.
    if has_then_keyword && body.len() == 1 {
        return;
    }

    let (else_line, _) = source.offset_to_line_col(else_node.else_keyword_loc().start_offset());
    let (stmt_line, stmt_col) = source.offset_to_line_col(first_stmt.location().start_offset());
    if stmt_line == else_line {
        diagnostics.push(cop.diagnostic(source, stmt_line, stmt_col, MSG.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ElseLayout, "cops/lint/else_layout");
}
