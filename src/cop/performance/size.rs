use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, HASH_NODE};

pub struct Size;

impl Cop for Size {
    fn name(&self) -> &'static str {
        "Performance/Size"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, HASH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"count" {
            return;
        }

        // Must have a receiver that is an Array or Hash literal (not AR relations etc.)
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Note: keyword_hash_node (keyword args like `foo(a: 1)`) intentionally not
        // handled — keyword hashes cannot be receivers of `.count`.
        if recv.as_array_node().is_none() && recv.as_hash_node().is_none() {
            return;
        }

        // Must have no arguments and no block
        if call.arguments().is_some() || call.block().is_some() {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, "Use `size` instead of `count`.".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Size, "cops/performance/size");
}
