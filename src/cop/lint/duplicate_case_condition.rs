use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DuplicateCaseCondition;

impl Cop for DuplicateCaseCondition {
    fn name(&self) -> &'static str {
        "Lint/DuplicateCaseCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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

        let mut seen = HashSet::new();
        let mut diagnostics = Vec::new();

        for when_node_ref in case_node.conditions().iter() {
            let when_node = match when_node_ref.as_when_node() {
                Some(w) => w,
                None => continue,
            };
            for condition in when_node.conditions().iter() {
                let loc = condition.location();
                let source_text = loc.as_slice();
                if !seen.insert(source_text.to_vec()) {
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Duplicate `when` condition detected.".to_string(),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateCaseCondition, "cops/lint/duplicate_case_condition");
}
