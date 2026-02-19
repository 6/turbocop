use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::ARRAY_NODE;

pub struct RedundantCapitalW;

impl Cop for RedundantCapitalW {
    fn name(&self) -> &'static str {
        "Style/RedundantCapitalW"
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
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let loc = node.location();
        let src_bytes = loc.as_slice();

        // Only check array nodes whose source starts with %W
        if !src_bytes.starts_with(b"%W") {
            return;
        }

        // Must be an array node
        if node.as_array_node().is_none() {
            return;
        }

        // Check if any element contains interpolation or special escape sequences
        if src_bytes.len() > 4 {
            let content = &src_bytes[3..src_bytes.len().saturating_sub(1)];
            let has_interpolation = content.windows(2).any(|w| w == b"#{");
            let has_escape = content.contains(&b'\\');

            if !has_interpolation && !has_escape {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use `%W` unless interpolation is needed. If not, use `%w`.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantCapitalW, "cops/style/redundant_capital_w");
}
