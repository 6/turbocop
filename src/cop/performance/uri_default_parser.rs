// Handles both as_constant_read_node and as_constant_path_node (qualified constants like ::URI)
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UriDefaultParser;

impl Cop for UriDefaultParser {
    fn name(&self) -> &'static str {
        "Performance/UriDefaultParser"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"decode" && method_name != b"encode" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_name = match constant_name(&receiver) {
            Some(n) => n,
            None => return Vec::new(),
        };

        if recv_name != b"URI" {
            return Vec::new();
        }

        let suggestion = if method_name == b"decode" {
            "URI::DEFAULT_PARSER.unescape"
        } else {
            "URI::DEFAULT_PARSER.escape"
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, format!(
            "Use `{suggestion}` instead of `URI.{}`.",
            std::str::from_utf8(method_name).unwrap_or("?")
        ))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(UriDefaultParser, "cops/performance/uri_default_parser");
}
