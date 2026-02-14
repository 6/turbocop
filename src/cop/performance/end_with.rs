use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct EndWith;

/// Check if regex content ends with \z and the prefix is a simple literal.
fn is_end_anchored_literal(content: &[u8]) -> bool {
    if content.len() < 3 {
        return false;
    }
    // Must end with \z
    if content[content.len() - 2] != b'\\' || content[content.len() - 1] != b'z' {
        return false;
    }
    let prefix = &content[..content.len() - 2];
    if prefix.is_empty() {
        return false;
    }
    for &b in prefix {
        match b {
            b'.' | b'*' | b'+' | b'?' | b'|' | b'(' | b')' | b'[' | b']' | b'{' | b'}'
            | b'^' | b'$' | b'\\' => return false,
            _ => {}
        }
    }
    true
}

impl Cop for EndWith {
    fn name(&self) -> &'static str {
        "Performance/EndWith"
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

        if call.name().as_slice() != b"match?" {
            return Vec::new();
        }

        if call.receiver().is_none() {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        let first_arg = match args.iter().next() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let regex_node = match first_arg.as_regular_expression_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let content = regex_node.content_loc().as_slice();
        if !is_end_anchored_literal(content) {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message:
                "Use `end_with?` instead of a regex match anchored to the end of the string."
                    .to_string(),
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
            &EndWith,
            include_bytes!("../../../testdata/cops/performance/end_with/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &EndWith,
            include_bytes!("../../../testdata/cops/performance/end_with/no_offense.rb"),
        );
    }
}
