use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
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

        let const_node = match receiver.as_constant_read_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if const_node.name().as_slice() != b"URI" {
            return Vec::new();
        }

        let suggestion = if method_name == b"decode" {
            "URI::DEFAULT_PARSER.unescape"
        } else {
            "URI::DEFAULT_PARSER.escape"
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: format!(
                "Use `{suggestion}` instead of `URI.{}`.",
                std::str::from_utf8(method_name).unwrap_or("?")
            ),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &UriDefaultParser,
            include_bytes!("../../../testdata/cops/performance/uri_default_parser/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &UriDefaultParser,
            include_bytes!("../../../testdata/cops/performance/uri_default_parser/no_offense.rb"),
        );
    }
}
