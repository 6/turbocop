use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{EMBEDDED_STATEMENTS_NODE, FALSE_NODE, FLOAT_NODE, INTEGER_NODE, NIL_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE};

pub struct LiteralInInterpolation;

impl Cop for LiteralInInterpolation {
    fn name(&self) -> &'static str {
        "Lint/LiteralInInterpolation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[EMBEDDED_STATEMENTS_NODE, FALSE_NODE, FLOAT_NODE, INTEGER_NODE, NIL_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let embedded = match node.as_embedded_statements_node() {
            Some(n) => n,
            None => return,
        };

        let stmts = match embedded.statements() {
            Some(s) => s,
            None => return,
        };

        let body = stmts.body();
        if body.len() != 1 {
            return;
        }

        let first = match body.first() {
            Some(n) => n,
            None => return,
        };

        // Skip whitespace-only string literals — `#{' '}` is a deliberate Ruby idiom
        // for preserving trailing whitespace in heredocs (Layout/TrailingWhitespace).
        if let Some(str_node) = first.as_string_node() {
            let content = str_node.content_loc().as_slice();
            if content.iter().all(|&b| b == b' ' || b == b'\t') {
                return;
            }
        }

        let is_literal = first.as_integer_node().is_some()
            || first.as_float_node().is_some()
            || first.as_string_node().is_some()
            || first.as_symbol_node().is_some()
            || first.as_nil_node().is_some()
            || first.as_true_node().is_some()
            || first.as_false_node().is_some();

        if !is_literal {
            return;
        }

        let loc = embedded.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Literal interpolation detected.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LiteralInInterpolation, "cops/lint/literal_in_interpolation");
}
