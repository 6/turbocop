use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingMethodEndStatement;

impl Cop for TrailingMethodEndStatement {
    fn name(&self) -> &'static str {
        "Style/TrailingMethodEndStatement"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Skip endless methods (def foo = ...)
        if def_node.equal_loc().is_some() {
            return Vec::new();
        }

        // Must have a body
        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Must be multiline
        let def_loc = def_node.location();
        let (def_start_line, _) = source.offset_to_line_col(def_loc.start_offset());
        let end_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };
        let (end_line, end_column) = source.offset_to_line_col(end_loc.start_offset());

        if def_start_line == end_line {
            return Vec::new();
        }

        // Check if body last line == end line
        let body_loc = body.location();
        let body_end_offset = body_loc.end_offset().saturating_sub(1);
        let (body_last_line, _) = source.offset_to_line_col(body_end_offset);

        if body_last_line == end_line {
            return vec![self.diagnostic(
                source,
                end_line,
                end_column,
                "Place the end statement of a multi-line method on its own line.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrailingMethodEndStatement, "cops/style/trailing_method_end_statement");
}
