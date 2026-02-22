// Handles both as_constant_read_node and as_constant_path_node (qualified constants like ::Fixnum)
use crate::cop::node_type::{CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UnifiedInteger;

impl Cop for UnifiedInteger {
    fn name(&self) -> &'static str {
        "Lint/UnifiedInteger"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        let name = match constant_name(node) {
            Some(n) => n,
            None => return,
        };

        let message = if name == b"Fixnum" {
            "Use `Integer` instead of `Fixnum`."
        } else if name == b"Bignum" {
            "Use `Integer` instead of `Bignum`."
        } else {
            return;
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let mut diag = self.diagnostic(source, line, column, message.to_string());
        if let Some(ref mut corr) = corrections {
            let src_bytes = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let replacement = if src_bytes.starts_with(b"::") {
                "::Integer".to_string()
            } else {
                "Integer".to_string()
            };
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
                cop_name: self.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnifiedInteger, "cops/lint/unified_integer");
    crate::cop_autocorrect_fixture_tests!(UnifiedInteger, "cops/lint/unified_integer");
}
