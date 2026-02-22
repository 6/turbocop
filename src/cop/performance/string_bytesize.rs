use crate::cop::node_type::{CALL_NODE, INTEGER_NODE};
use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct StringBytesize;

impl Cop for StringBytesize {
    fn name(&self) -> &'static str {
        "Performance/StringBytesize"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE]
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

        // Must be .bytes.{size,length,count}
        if chain.inner_method != b"bytes" {
            return;
        }

        match chain.outer_method {
            b"size" | b"length" | b"count" => {}
            _ => return,
        }

        // The inner call (.bytes) must have a receiver
        let recv = match chain.inner_call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver must not be nil (bare `bytes`) or an integer
        if recv.as_integer_node().is_some() {
            return;
        }

        // Report on the `.bytes.{size,length,count}` portion
        // Offense range starts at the `.bytes` selector
        let inner_loc = chain.inner_call.message_loc().unwrap();
        let outer_call = node.as_call_node().unwrap();
        let outer_end = outer_call.location().end_offset();
        let start = inner_loc.start_offset();

        let (line, column) = source.offset_to_line_col(start);
        let end_col_offset = outer_end - start;
        // Build annotation-compatible offset
        let _ = end_col_offset;

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `String#bytesize` instead of calculating the size of the bytes array.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringBytesize, "cops/performance/string_bytesize");
}
