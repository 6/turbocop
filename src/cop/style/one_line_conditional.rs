use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct OneLineConditional;

impl Cop for OneLineConditional {
    fn name(&self) -> &'static str {
        "Style/OneLineConditional"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check `if ... then ... else ... end` on one line
        if let Some(if_node) = node.as_if_node() {
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(), // ternary
            };

            let kw_bytes = kw_loc.as_slice();
            if kw_bytes != b"if" {
                return Vec::new();
            }

            // Must not be modifier form
            if if_node.end_keyword_loc().is_none() {
                return Vec::new();
            }

            // Must have a then keyword
            if if_node.then_keyword_loc().is_none() {
                return Vec::new();
            }

            // Must have an else branch
            if if_node.subsequent().is_none() {
                return Vec::new();
            }

            // Must be single-line
            let loc = if_node.location();
            let (start_line, _) = source.offset_to_line_col(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
            if start_line != end_line {
                return Vec::new();
            }

            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Favor the ternary operator (`?:`) over single-line `if/then/else/end` constructs.".to_string(),
            )];
        }

        // Check `unless ... then ... else ... end` on one line
        if let Some(unless_node) = node.as_unless_node() {
            let kw_loc = unless_node.keyword_loc();
            if kw_loc.as_slice() != b"unless" {
                return Vec::new();
            }

            // Must not be modifier form
            if unless_node.end_keyword_loc().is_none() {
                return Vec::new();
            }

            // Must have a then keyword
            if unless_node.then_keyword_loc().is_none() {
                return Vec::new();
            }

            // Must have an else branch
            if unless_node.else_clause().is_none() {
                return Vec::new();
            }

            // Must be single-line
            let loc = unless_node.location();
            let (start_line, _) = source.offset_to_line_col(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
            if start_line != end_line {
                return Vec::new();
            }

            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Favor the ternary operator (`?:`) over single-line `unless/then/else/end` constructs.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OneLineConditional, "cops/style/one_line_conditional");
}
