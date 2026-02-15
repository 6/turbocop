use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyConditionalBody;

/// Check if there are any comments within a byte offset range.
fn has_comment_in_range(parse_result: &ruby_prism::ParseResult<'_>, start: usize, end: usize) -> bool {
    for comment in parse_result.comments() {
        let comment_start = comment.location().start_offset();
        if comment_start >= start && comment_start < end {
            return true;
        }
    }
    false
}

impl Cop for EmptyConditionalBody {
    fn name(&self) -> &'static str {
        "Lint/EmptyConditionalBody"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_comments = config.get_bool("AllowComments", true);

        // Check IfNode
        if let Some(if_node) = node.as_if_node() {
            // Only check keyword if, not ternaries
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(),
            };

            let body_empty = match if_node.statements() {
                None => true,
                Some(stmts) => stmts.body().is_empty(),
            };

            if body_empty {
                if allow_comments {
                    // Check if there are comments anywhere within the if/elsif node.
                    // RuboCop considers any comment within the node range (including
                    // inside the predicate) as sufficient to skip the offense.
                    let range_start = kw_loc.start_offset();
                    let range_end = if let Some(sub) = if_node.subsequent() {
                        sub.location().start_offset()
                    } else if let Some(end_kw) = if_node.end_keyword_loc() {
                        end_kw.start_offset()
                    } else {
                        node.location().end_offset()
                    };
                    if has_comment_in_range(parse_result, range_start, range_end) {
                        return Vec::new();
                    }
                }
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid empty `if` conditions.".to_string(),
                )];
            }
        }

        // Check UnlessNode
        if let Some(unless_node) = node.as_unless_node() {
            let body_empty = match unless_node.statements() {
                None => true,
                Some(stmts) => stmts.body().is_empty(),
            };

            if body_empty {
                if allow_comments {
                    let body_start = unless_node.predicate().location().end_offset();
                    let body_end = if let Some(else_clause) = unless_node.else_clause() {
                        else_clause.location().start_offset()
                    } else if let Some(end_kw) = unless_node.end_keyword_loc() {
                        end_kw.start_offset()
                    } else {
                        node.location().end_offset()
                    };
                    if has_comment_in_range(parse_result, body_start, body_end) {
                        return Vec::new();
                    }
                }
                let kw_loc = unless_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid empty `unless` conditions.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyConditionalBody, "cops/lint/empty_conditional_body");
}
