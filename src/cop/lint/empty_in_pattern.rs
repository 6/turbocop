use crate::cop::node_type::{CASE_MATCH_NODE, IN_NODE};
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[CASE_MATCH_NODE, IN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_comments = config.get_bool("AllowComments", true);

        // CaseMatchNode represents `case ... in ... end` (pattern matching)
        let case_match = match node.as_case_match_node() {
            Some(n) => n,
            None => return,
        };

        let conditions: Vec<_> = case_match.conditions().iter().collect();
        for (i, condition) in conditions.iter().enumerate() {
            if let Some(in_node) = condition.as_in_node() {
                // Check if the body is empty
                let body_empty = in_node.statements().is_none()
                    || in_node.statements().is_none_or(|s| s.body().is_empty());

                if body_empty {
                    // When AllowComments is true, check if the source between
                    // this in-pattern and the next clause contains comment lines
                    if allow_comments {
                        let search_start = in_node.pattern().location().end_offset();
                        let search_end = if i + 1 < conditions.len() {
                            conditions[i + 1].location().start_offset()
                        } else if let Some(else_clause) = case_match.else_clause() {
                            else_clause.location().start_offset()
                        } else {
                            case_match.end_keyword_loc().start_offset()
                        };
                        let src_bytes = source.as_bytes();
                        let end = search_end.min(src_bytes.len());
                        if search_start < end {
                            let body_bytes = &src_bytes[search_start..end];
                            let has_comment = body_bytes.split(|&b| b == b'\n').any(|line| {
                                let trimmed = line
                                    .iter()
                                    .skip_while(|&&b| b == b' ' || b == b'\t')
                                    .copied()
                                    .collect::<Vec<_>>();
                                trimmed.starts_with(b"#")
                            });
                            if has_comment {
                                continue;
                            }
                        }
                    }

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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyInPattern, "cops/lint/empty_in_pattern");
}
