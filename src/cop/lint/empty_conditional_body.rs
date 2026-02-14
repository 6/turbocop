use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyConditionalBody;

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
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
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
