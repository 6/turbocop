use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct UnfreezeString;

impl Cop for UnfreezeString {
    fn name(&self) -> &'static str {
        "Performance/UnfreezeString"
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

        if call.name().as_slice() != b"new" {
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

        if const_node.name().as_slice() != b"String" {
            return Vec::new();
        }

        // Allow String.new with no args, or String.new('') (empty string)
        match call.arguments() {
            None => {} // String.new — flag it
            Some(arguments) => {
                let args = arguments.arguments();
                if args.len() != 1 {
                    return Vec::new();
                }
                // Must be a string node with empty content
                let first_arg = match args.iter().next() {
                    Some(a) => a,
                    None => return Vec::new(),
                };
                match first_arg.as_string_node() {
                    Some(s) => {
                        if !s.unescaped().is_empty() {
                            return Vec::new();
                        }
                    }
                    None => return Vec::new(),
                }
            }
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use unary plus to get an unfrozen string literal.".to_string(),
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
            &UnfreezeString,
            include_bytes!("../../../testdata/cops/performance/unfreeze_string/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &UnfreezeString,
            include_bytes!("../../../testdata/cops/performance/unfreeze_string/no_offense.rb"),
        );
    }
}
