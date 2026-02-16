use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct BarePercentLiterals;

impl Cop for BarePercentLiterals {
    fn name(&self) -> &'static str {
        "Style/BarePercentLiterals"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "bare_percent");

        // Check both StringNode and InterpolatedStringNode
        let (opening_loc, node_loc) = if let Some(s) = node.as_string_node() {
            (s.opening_loc(), s.location())
        } else if let Some(s) = node.as_interpolated_string_node() {
            (s.opening_loc(), s.location())
        } else {
            return Vec::new();
        };

        let opening = match opening_loc {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let opening_bytes = opening.as_slice();

        match enforced_style {
            "bare_percent" => {
                // Flag %Q usage
                if opening_bytes.starts_with(b"%Q") {
                    let (line, column) = source.offset_to_line_col(node_loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `%` instead of `%Q`.".to_string(),
                    )];
                }
            }
            "percent_q" => {
                // Flag bare % usage (not %q, %Q, etc. - just % followed by a non-alpha)
                if opening_bytes.starts_with(b"%")
                    && !opening_bytes.starts_with(b"%Q")
                    && !opening_bytes.starts_with(b"%q")
                    && opening_bytes.len() >= 2
                    && !opening_bytes[1].is_ascii_alphabetic()
                {
                    let (line, column) = source.offset_to_line_col(node_loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `%Q` instead of `%`.".to_string(),
                    )];
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BarePercentLiterals, "cops/style/bare_percent_literals");
}
