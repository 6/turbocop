use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct PercentQLiterals;

impl Cop for PercentQLiterals {
    fn name(&self) -> &'static str {
        "Style/PercentQLiterals"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "lower_case_q");

        // Check for %Q or %q string nodes
        // In Prism, %Q strings are InterpolatedStringNode, %q strings are StringNode
        // We need to check the source text to see if it starts with %Q or %q

        let loc = node.location();
        let src_bytes = loc.as_slice();

        if node.as_string_node().is_some() || node.as_interpolated_string_node().is_some() {
            if style == "lower_case_q" {
                // Flag %Q when %q would suffice (no interpolation)
                if src_bytes.starts_with(b"%Q") {
                    // Check if there's interpolation
                    if node.as_string_node().is_some() {
                        // StringNode means no interpolation -> should use %q
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Use `%q` instead of `%Q`.".to_string(),
                        )];
                    }
                }
            } else if style == "upper_case_q" {
                // Flag %q when %Q is preferred
                if src_bytes.starts_with(b"%q") {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `%Q` instead of `%q`.".to_string(),
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
    crate::cop_fixture_tests!(PercentQLiterals, "cops/style/percent_q_literals");
}
