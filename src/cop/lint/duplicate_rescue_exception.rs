use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::BEGIN_NODE;

pub struct DuplicateRescueException;

impl Cop for DuplicateRescueException {
    fn name(&self) -> &'static str {
        "Lint/DuplicateRescueException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE]
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
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return,
        };

        let mut seen = HashSet::new();
        let mut rescue_opt = begin_node.rescue_clause();

        while let Some(rescue_node) = rescue_opt {
            for exception in rescue_node.exceptions().iter() {
                let text = exception.location().as_slice().to_vec();
                if !seen.insert(text) {
                    let loc = exception.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Duplicate `rescue` exception detected.".to_string(),
                    ));
                }
            }
            rescue_opt = rescue_node.subsequent();
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateRescueException, "cops/lint/duplicate_rescue_exception");
}
