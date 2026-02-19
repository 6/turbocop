use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantSplatExpansion;

impl Cop for RedundantSplatExpansion {
    fn name(&self) -> &'static str {
        "Lint/RedundantSplatExpansion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_percent = config.get_bool("AllowPercentLiteralArrayArgument", true);

        let splat = match node.as_splat_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let child = match splat.expression() {
            Some(e) => e,
            None => return Vec::new(),
        };

        // Check if the splat is on a literal: array, string, integer, float
        let is_literal = child.as_array_node().is_some()
            || child.as_string_node().is_some()
            || child.as_integer_node().is_some()
            || child.as_float_node().is_some()
            || child.as_interpolated_string_node().is_some();

        if !is_literal {
            return Vec::new();
        }

        // When AllowPercentLiteralArrayArgument is true (default), skip
        // percent literal arrays used as method arguments.
        if allow_percent {
            if let Some(array_node) = child.as_array_node() {
                // Check if it's a percent literal (%w, %W, %i, %I)
                if let Some(open_loc) = array_node.opening_loc() {
                    let open = open_loc.as_slice();
                    if open.starts_with(b"%w") || open.starts_with(b"%W")
                        || open.starts_with(b"%i") || open.starts_with(b"%I")
                    {
                        // This is a percent literal array — skip it.
                        // RuboCop only skips when it's a method argument or part of
                        // an array, but we conservatively skip all percent literal
                        // splats when the config allows it.
                        return Vec::new();
                    }
                }
            }
        }

        let loc = splat.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Replace splat expansion with comma separated values.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSplatExpansion, "cops/lint/redundant_splat_expansion");
}
