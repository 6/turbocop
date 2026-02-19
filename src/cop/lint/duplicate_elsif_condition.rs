use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::IF_NODE;

pub struct DuplicateElsifCondition;

impl Cop for DuplicateElsifCondition {
    fn name(&self) -> &'static str {
        "Lint/DuplicateElsifCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Only process top-level if (not elsif nodes visited separately)
        // The walker visits all IfNodes including elsif ones, so we need
        // to check this is a top-level if by checking there's an if_keyword
        let kw_loc = if_node.if_keyword_loc();
        if kw_loc.is_none() {
            return Vec::new();
        }
        let kw_slice = kw_loc.unwrap().as_slice();
        if kw_slice != b"if" && kw_slice != b"unless" {
            return Vec::new();
        }

        let mut seen = HashSet::new();
        let mut diagnostics = Vec::new();

        // Add the first condition
        let first_cond = if_node.predicate().location().as_slice().to_vec();
        seen.insert(first_cond);

        // Walk elsif chain
        let mut subsequent = if_node.subsequent();
        while let Some(sub) = subsequent {
            if let Some(elsif) = sub.as_if_node() {
                let cond_text = elsif.predicate().location().as_slice().to_vec();
                if !seen.insert(cond_text) {
                    let loc = elsif.predicate().location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Duplicate `elsif` condition detected.".to_string(),
                    ));
                }
                subsequent = elsif.subsequent();
            } else {
                break;
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateElsifCondition, "cops/lint/duplicate_elsif_condition");
}
