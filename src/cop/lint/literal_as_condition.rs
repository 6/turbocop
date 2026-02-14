use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct LiteralAsCondition;

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    matches!(
        node,
        ruby_prism::Node::TrueNode { .. }
            | ruby_prism::Node::FalseNode { .. }
            | ruby_prism::Node::NilNode { .. }
            | ruby_prism::Node::IntegerNode { .. }
            | ruby_prism::Node::FloatNode { .. }
            | ruby_prism::Node::RationalNode { .. }
            | ruby_prism::Node::ImaginaryNode { .. }
    )
}

impl Cop for LiteralAsCondition {
    fn name(&self) -> &'static str {
        "Lint/LiteralAsCondition"
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
        // Try IfNode
        if let Some(if_node) = node.as_if_node() {
            let predicate = if_node.predicate();
            if is_literal(&predicate) {
                // Use if_keyword_loc to get the keyword position; skip ternaries
                if let Some(kw_loc) = if_node.if_keyword_loc() {
                    let literal_text =
                        std::str::from_utf8(predicate.location().as_slice()).unwrap_or("literal");
                    let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                    return vec![Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: self.default_severity(),
                        cop_name: self.name().to_string(),
                        message: format!("Literal `{literal_text}` appeared as a condition."),
                    }];
                }
            }
        }

        // Try WhileNode
        if let Some(while_node) = node.as_while_node() {
            let predicate = while_node.predicate();
            if is_literal(&predicate) {
                let kw_loc = while_node.keyword_loc();
                let literal_text =
                    std::str::from_utf8(predicate.location().as_slice()).unwrap_or("literal");
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: self.default_severity(),
                    cop_name: self.name().to_string(),
                    message: format!("Literal `{literal_text}` appeared as a condition."),
                }];
            }
        }

        // Try UntilNode
        if let Some(until_node) = node.as_until_node() {
            let predicate = until_node.predicate();
            if is_literal(&predicate) {
                let kw_loc = until_node.keyword_loc();
                let literal_text =
                    std::str::from_utf8(predicate.location().as_slice()).unwrap_or("literal");
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: self.default_severity(),
                    cop_name: self.name().to_string(),
                    message: format!("Literal `{literal_text}` appeared as a condition."),
                }];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &LiteralAsCondition,
            include_bytes!("../../../testdata/cops/lint/literal_as_condition/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &LiteralAsCondition,
            include_bytes!("../../../testdata/cops/lint/literal_as_condition/no_offense.rb"),
        );
    }
}
