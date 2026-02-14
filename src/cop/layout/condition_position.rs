use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct ConditionPosition;

impl Cop for ConditionPosition {
    fn name(&self) -> &'static str {
        "Layout/ConditionPosition"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        if let Some(if_node) = node.as_if_node() {
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(),
            };
            let keyword = if kw_loc.as_slice() == b"if" {
                "if"
            } else {
                // unless - still check
                "unless"
            };
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let predicate = if_node.predicate();
            let (pred_line, pred_col) =
                source.offset_to_line_col(predicate.location().start_offset());
            if pred_line != kw_line {
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: pred_line,
                        column: pred_col,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: format!("Place the condition on the same line as `{keyword}`."),
                }];
            }
        } else if let Some(while_node) = node.as_while_node() {
            let kw_loc = while_node.keyword_loc();
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let predicate = while_node.predicate();
            let (pred_line, pred_col) =
                source.offset_to_line_col(predicate.location().start_offset());
            if pred_line != kw_line {
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: pred_line,
                        column: pred_col,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Place the condition on the same line as `while`.".to_string(),
                }];
            }
        } else if let Some(until_node) = node.as_until_node() {
            let kw_loc = until_node.keyword_loc();
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let predicate = until_node.predicate();
            let (pred_line, pred_col) =
                source.offset_to_line_col(predicate.location().start_offset());
            if pred_line != kw_line {
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: pred_line,
                        column: pred_col,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Place the condition on the same line as `until`.".to_string(),
                }];
            }
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
            &ConditionPosition,
            include_bytes!("../../../testdata/cops/layout/condition_position/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &ConditionPosition,
            include_bytes!("../../../testdata/cops/layout/condition_position/no_offense.rb"),
        );
    }

    #[test]
    fn inline_if_no_offense() {
        let source = b"x = 1 if true\n";
        let diags = run_cop_full(&ConditionPosition, source);
        assert!(diags.is_empty());
    }
}
