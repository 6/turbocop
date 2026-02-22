use crate::cop::node_type::HASH_NODE;
use crate::cop::util::has_trailing_comma;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingCommaInHashLiteral;

impl Cop for TrailingCommaInHashLiteral {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInHashLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[HASH_NODE]
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
        // Note: keyword_hash_node (keyword args like `foo(a: 1)`) intentionally not
        // handled — this cop only applies to trailing commas in hash literals.
        let hash_node = match node.as_hash_node() {
            Some(h) => h,
            None => return,
        };

        let closing_loc = hash_node.closing_loc();
        let elements = hash_node.elements();
        let last_elem = match elements.last() {
            Some(e) => e,
            None => return,
        };

        let last_end = last_elem.location().end_offset();
        let closing_start = closing_loc.start_offset();
        let bytes = source.as_bytes();
        let has_comma = has_trailing_comma(bytes, last_end, closing_start);

        let style = config.get_str("EnforcedStyleForMultiline", "no_comma");
        let last_line = source.offset_to_line_col(last_end).0;
        let close_line = source.offset_to_line_col(closing_start).0;
        let is_multiline = close_line > last_line;

        match style {
            "comma" | "consistent_comma" => {
                // Require trailing comma in multiline; no opinion on single-line
                if is_multiline && !has_comma {
                    let (line, column) = source.offset_to_line_col(last_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Put a comma after the last item of a multiline hash.".to_string(),
                    ));
                }
            }
            _ => {
                // no_comma: flag trailing commas in multiline
                if has_comma {
                    let search_range = &bytes[last_end..closing_start];
                    if let Some(comma_offset) = search_range.iter().position(|&b| b == b',') {
                        let abs_offset = last_end + comma_offset;
                        let (line, column) = source.offset_to_line_col(abs_offset);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid comma after the last item of a hash.".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        TrailingCommaInHashLiteral,
        "cops/style/trailing_comma_in_hash_literal"
    );
}
