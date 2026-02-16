use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyInPattern;

impl Cop for EmptyInPattern {
    fn name(&self) -> &'static str {
        "Lint/EmptyInPattern"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _allow_comments = config.get_bool("AllowComments", true);

        // CaseMatchNode represents `case ... in ... end` (pattern matching)
        let case_match = match node.as_case_match_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        for condition in case_match.conditions().iter() {
            if let Some(in_node) = condition.as_in_node() {
                // Check if the body is empty
                let body_empty = in_node.statements().is_none()
                    || in_node
                        .statements()
                        .map_or(true, |s| s.body().is_empty());

                if body_empty {
                    let loc = in_node.in_loc();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid `in` branches without a body.".to_string(),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyInPattern, "cops/lint/empty_in_pattern");
}
