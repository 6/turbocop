use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InverseMethods;

impl Cop for InverseMethods {
    fn name(&self) -> &'static str {
        "Style/InverseMethods"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_bytes = call.name().as_slice();

        let _inverse_methods = config.get_string_hash("InverseMethods");
        let _inverse_blocks = config.get_string_hash("InverseBlocks");

        // Check for !foo.select { } -> foo.reject { }
        // Check for !foo.any? -> foo.none?
        // Default inverse methods: select <-> reject, any? <-> none?, include? <-> exclude?

        // Pattern: !receiver.method - the call is `!` with the inner being a method call
        if method_bytes != b"!" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let inner_method = inner_call.name().as_slice();

        let inverse = match inner_method {
            b"any?" => Some("none?"),
            b"none?" => Some("any?"),
            b"include?" => Some("exclude?"),
            b"exclude?" => Some("include?"),
            b"even?" => Some("odd?"),
            b"odd?" => Some("even?"),
            b"present?" => Some("blank?"),
            b"blank?" => Some("present?"),
            b"empty?" => Some("any?"),
            _ => None,
        };

        if let Some(inv) = inverse {
            let inner_name = std::str::from_utf8(inner_method).unwrap_or("method");
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Use `{}` instead of inverting `{}`.", inv, inner_name),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InverseMethods, "cops/style/inverse_methods");
}
