use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Casecmp;

impl Cop for Casecmp {
    fn name(&self) -> &'static str {
        "Performance/Casecmp"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.outer_method != b"==" {
            return;
        }

        if chain.inner_method != b"downcase" && chain.inner_method != b"upcase" {
            return;
        }

        // downcase/upcase should have no arguments
        if chain.inner_call.arguments().is_some() {
            return;
        }

        // Skip safe navigation (&.) — casecmp doesn't handle nil safely
        if let Some(op) = chain.inner_call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return;
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `casecmp` instead of `{} ==`.",
                std::str::from_utf8(chain.inner_method).unwrap_or("downcase")
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Casecmp, "cops/performance/casecmp");
}
