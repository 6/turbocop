use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct Lambda;

impl Cop for Lambda {
    fn name(&self) -> &'static str {
        "Style/Lambda"
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

        // Only bare `lambda` calls (no receiver)
        if call.receiver().is_some() {
            return Vec::new();
        }

        if call.name().as_slice() != b"lambda" {
            return Vec::new();
        }

        let loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: Severity::Convention,
            cop_name: self.name().to_string(),
            message: "Use the `-> {}` lambda literal syntax for all lambdas.".to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &Lambda,
            include_bytes!("../../../testdata/cops/style/lambda/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &Lambda,
            include_bytes!("../../../testdata/cops/style/lambda/no_offense.rb"),
        );
    }

    #[test]
    fn lambda_with_receiver_is_ignored() {
        let source = b"obj.lambda { |x| x }\n";
        let diags = run_cop_full(&Lambda, source);
        assert!(diags.is_empty());
    }
}
