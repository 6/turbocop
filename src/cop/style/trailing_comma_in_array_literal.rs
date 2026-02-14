use crate::cop::util::has_trailing_comma;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingCommaInArrayLiteral;

impl Cop for TrailingCommaInArrayLiteral {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInArrayLiteral"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let closing_loc = match array_node.closing_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let elements = array_node.elements();
        let last_elem = match elements.last() {
            Some(e) => e,
            None => return Vec::new(),
        };

        let last_end = last_elem.location().end_offset();
        let closing_start = closing_loc.start_offset();
        let bytes = source.as_bytes();

        if has_trailing_comma(bytes, last_end, closing_start) {
            let search_range = &bytes[last_end..closing_start];
            if let Some(comma_offset) = search_range.iter().position(|&b| b == b',') {
                let abs_offset = last_end + comma_offset;
                let (line, column) = source.offset_to_line_col(abs_offset);
                return vec![self.diagnostic(source, line, column, "Avoid comma after the last item of an array.".to_string())];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(TrailingCommaInArrayLiteral, "cops/style/trailing_comma_in_array_literal");
}
