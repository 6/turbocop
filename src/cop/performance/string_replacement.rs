use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct StringReplacement;

impl Cop for StringReplacement {
    fn name(&self) -> &'static str {
        "Performance/StringReplacement"
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

        if call.name().as_slice() != b"gsub" {
            return Vec::new();
        }

        // Must have a receiver (str.gsub)
        if call.receiver().is_none() {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        if args.len() != 2 {
            return Vec::new();
        }

        let mut args_iter = args.iter();
        let first_node = match args_iter.next() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let second_node = match args_iter.next() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let first = match first_node.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let second = match second_node.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Both must be single-character strings
        if first.unescaped().len() != 1 || second.unescaped().len() != 1 {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use `tr` instead of `gsub` when replacing single characters.".to_string(),
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
            &StringReplacement,
            include_bytes!("../../../testdata/cops/performance/string_replacement/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &StringReplacement,
            include_bytes!("../../../testdata/cops/performance/string_replacement/no_offense.rb"),
        );
    }
}
