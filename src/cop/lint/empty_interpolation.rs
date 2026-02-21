use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::EMBEDDED_STATEMENTS_NODE;

pub struct EmptyInterpolation;

impl Cop for EmptyInterpolation {
    fn name(&self) -> &'static str {
        "Lint/EmptyInterpolation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[EMBEDDED_STATEMENTS_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let embedded = match node.as_embedded_statements_node() {
            Some(n) => n,
            None => return,
        };

        let body_empty = match embedded.statements() {
            None => true,
            Some(stmts) => stmts.body().is_empty(),
        };

        if !body_empty {
            return;
        }

        let loc = embedded.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(
            source, line, column,
            "Empty interpolation detected.".to_string(),
        );
        if let Some(ref mut corr) = corrections {
            corr.push(crate::correction::Correction {
                start: loc.start_offset(), end: loc.end_offset(),
                replacement: String::new(),
                cop_name: self.name(), cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyInterpolation, "cops/lint/empty_interpolation");
    crate::cop_autocorrect_fixture_tests!(EmptyInterpolation, "cops/lint/empty_interpolation");
}
