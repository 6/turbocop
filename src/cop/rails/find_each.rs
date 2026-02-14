use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FindEach;

const AR_SCOPE_METHODS: &[&[u8]] = &[b"all", b"where", b"order", b"select"];

impl Cop for FindEach {
    fn name(&self) -> &'static str {
        "Rails/FindEach"
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

        if chain.outer_method != b"each" {
            return Vec::new();
        }

        if !AR_SCOPE_METHODS.contains(&chain.inner_method) {
            return Vec::new();
        }

        // The outer call (each) should have a block
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if outer_call.block().is_none() {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `find_each` instead of `each` for batch processing.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FindEach, "cops/rails/find_each");
}
