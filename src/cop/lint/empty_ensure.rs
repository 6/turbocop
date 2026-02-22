use crate::cop::node_type::BEGIN_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyEnsure;

impl Cop for EmptyEnsure {
    fn name(&self) -> &'static str {
        "Lint/EmptyEnsure"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE]
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
        // EnsureNode is not visited directly by the generic walker.
        // It appears as ensure_clause() on BeginNode.
        let ensure_node = if let Some(begin_node) = node.as_begin_node() {
            begin_node.ensure_clause()
        } else {
            None
        };

        let ensure_node = match ensure_node {
            Some(n) => n,
            None => return,
        };

        let body_empty = match ensure_node.statements() {
            None => true,
            Some(stmts) => stmts.body().is_empty(),
        };

        if !body_empty {
            return;
        }

        let kw_loc = ensure_node.ensure_keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Empty `ensure` block detected.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyEnsure, "cops/lint/empty_ensure");
}
