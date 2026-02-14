use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct YodaCondition;

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
}

impl Cop for YodaCondition {
    fn name(&self) -> &'static str {
        "Style/YodaCondition"
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

        let name = call.name().as_slice();
        if name != b"==" && name != b"!=" {
            return Vec::new();
        }

        // receiver is the left side
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        // Yoda: literal on left, non-literal on right
        if is_literal(&receiver) && !is_literal(&arg_list[0]) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Prefer non-Yoda conditions.".to_string(),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &YodaCondition,
            include_bytes!("../../../testdata/cops/style/yoda_condition/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &YodaCondition,
            include_bytes!("../../../testdata/cops/style/yoda_condition/no_offense.rb"),
        );
    }

    #[test]
    fn both_literals_not_flagged() {
        let source = b"1 == 1\n";
        let diags = run_cop_full(&YodaCondition, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn nil_on_left_is_flagged() {
        let source = b"nil == x\n";
        let diags = run_cop_full(&YodaCondition, source);
        assert_eq!(diags.len(), 1);
    }
}
