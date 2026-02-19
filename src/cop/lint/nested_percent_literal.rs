use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::ARRAY_NODE;

pub struct NestedPercentLiteral;

/// Percent literal prefixes that indicate a nested percent literal.
const PERCENT_PREFIXES: &[&[u8]] = &[
    b"%w", b"%W", b"%i", b"%I", b"%q", b"%Q", b"%r", b"%s", b"%x",
];

impl Cop for NestedPercentLiteral {
    fn name(&self) -> &'static str {
        "Lint/NestedPercentLiteral"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
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

        // Check if this is a %w or %i literal
        let open_loc = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let open_src = open_loc.as_slice();
        let is_percent_literal = open_src.starts_with(b"%w")
            || open_src.starts_with(b"%W")
            || open_src.starts_with(b"%i")
            || open_src.starts_with(b"%I");

        if !is_percent_literal {
            return Vec::new();
        }

        // Check if any element contains a percent literal prefix
        for element in array_node.elements().iter() {
            let elem_loc = element.location();
            let elem_src = &source.as_bytes()[elem_loc.start_offset()..elem_loc.end_offset()];

            for prefix in PERCENT_PREFIXES {
                if elem_src.len() > prefix.len()
                    && elem_src.starts_with(prefix)
                    && !elem_src[prefix.len()..prefix.len() + 1]
                        .first()
                        .map_or(true, |b| b.is_ascii_alphanumeric())
                {
                    let loc = array_node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Within percent literals, nested percent literals do not function and may be unwanted in the result.".to_string(),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedPercentLiteral, "cops/lint/nested_percent_literal");
}
