use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct UriEscapeUnescape;

impl Cop for UriEscapeUnescape {
    fn name(&self) -> &'static str {
        "Lint/UriEscapeUnescape"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        let is_escape = method_name == b"escape";
        let is_unescape = method_name == b"unescape";
        if !is_escape && !is_unescape {
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

        let message = if is_escape {
            "`URI.escape` method is obsolete and should not be used."
        } else {
            "`URI.unescape` method is obsolete and should not be used."
        };

        let msg_loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: message.to_string(),
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
            &UriEscapeUnescape,
            include_bytes!("../../../testdata/cops/lint/uri_escape_unescape/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &UriEscapeUnescape,
            include_bytes!("../../../testdata/cops/lint/uri_escape_unescape/no_offense.rb"),
        );
    }
}
