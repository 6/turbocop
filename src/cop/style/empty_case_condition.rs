use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CASE_NODE;

pub struct EmptyCaseCondition;

impl Cop for EmptyCaseCondition {
    fn name(&self) -> &'static str {
        "Style/EmptyCaseCondition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CASE_NODE]
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
        let case_node = match node.as_case_node() {
            Some(c) => c,
            None => return,
        };

        // Only flag if case has no predicate (empty case condition)
        if case_node.predicate().is_some() {
            return;
        }

        // Don't flag if the case is used as a value (assigned, returned, or passed as argument).
        // We check if the `case` keyword is NOT at the beginning of the line (i.e.,
        // something precedes it on the same line like `v = case` or `return case`).
        let case_kw_loc = case_node.case_keyword_loc();
        let case_offset = case_kw_loc.start_offset();
        let (case_line, _) = source.offset_to_line_col(case_offset);

        // Get the line text to check what precedes `case`
        let lines: Vec<_> = source.lines().collect();
        if case_line > 0 && case_line <= lines.len() {
            let line_text = match std::str::from_utf8(lines[case_line - 1]) {
                Ok(s) => s,
                Err(_) => "",
            };
            let trimmed = line_text.trim();
            // If the line doesn't start with `case`, something precedes it
            if !trimmed.starts_with("case") {
                return;
            }
        }

        let (line, column) = source.offset_to_line_col(case_offset);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not use empty `case` condition, instead use an `if` expression.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyCaseCondition, "cops/style/empty_case_condition");
}
