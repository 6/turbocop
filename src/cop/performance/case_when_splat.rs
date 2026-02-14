use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct CaseWhenSplat;

impl Cop for CaseWhenSplat {
    fn name(&self) -> &'static str {
        "Performance/CaseWhenSplat"
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
    ) -> Vec<Diagnostic> {
        let case_node = match node.as_case_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        for when_node_ref in case_node.conditions().iter() {
            let when_node = match when_node_ref.as_when_node() {
                Some(w) => w,
                None => continue,
            };

            for condition in when_node.conditions().iter() {
                if condition.as_splat_node().is_some() {
                    let loc = when_node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, "Reorder `when` conditions with a splat to the end.".to_string()));
                    break; // One diagnostic per when clause
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CaseWhenSplat, "cops/performance/case_when_splat");
}
