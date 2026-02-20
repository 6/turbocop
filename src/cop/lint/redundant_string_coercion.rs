use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, EMBEDDED_STATEMENTS_NODE, INTERPOLATED_STRING_NODE};

pub struct RedundantStringCoercion;

impl Cop for RedundantStringCoercion {
    fn name(&self) -> &'static str {
        "Lint/RedundantStringCoercion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, EMBEDDED_STATEMENTS_NODE, INTERPOLATED_STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let interp = match node.as_interpolated_string_node() {
            Some(n) => n,
            None => return,
        };

        let parts = interp.parts();

        // We look for EmbeddedStatementsNode parts containing a to_s call

        for part in &parts {
            let embedded = match part.as_embedded_statements_node() {
                Some(e) => e,
                None => continue,
            };

            let statements = match embedded.statements() {
                Some(s) => s,
                None => continue,
            };

            let body = statements.body();
            if body.len() != 1 {
                continue;
            }

            let first = match body.first() {
                Some(n) => n,
                None => continue,
            };

            let call = match first.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            if call.name().as_slice() != b"to_s" {
                continue;
            }

            // Ensure to_s has no arguments
            if call.arguments().is_some() {
                continue;
            }

            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant use of `Object#to_s` in interpolation.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantStringCoercion, "cops/lint/redundant_string_coercion");
}
