use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantStringChars;

impl Cop for RedundantStringChars {
    fn name(&self) -> &'static str {
        "Performance/RedundantStringChars"
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
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        if chain.inner_method != b"chars" {
            return Vec::new();
        }

        // The inner call must have a receiver (str.chars)
        if chain.inner_call.receiver().is_none() {
            return Vec::new();
        }

        // outer method must be `first`, `last`, or `[]`
        if chain.outer_method != b"first"
            && chain.outer_method != b"last"
            && chain.outer_method != b"[]"
        {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `[]` instead of `chars.first`.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RedundantStringChars, "cops/performance/redundant_string_chars");
}
