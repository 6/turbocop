use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct SpaceAroundEqualsInParameterDefault;

impl Cop for SpaceAroundEqualsInParameterDefault {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundEqualsInParameterDefault"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let opt = match node.as_optional_parameter_node() {
            Some(o) => o,
            None => return Vec::new(),
        };

        let enforced = config
            .options
            .get("EnforcedStyle")
            .and_then(|v| v.as_str())
            .unwrap_or("space");

        let op = opt.operator_loc();
        let bytes = source.as_bytes();
        let op_start = op.start_offset();
        let op_end = op.end_offset();

        let space_before = op_start > 0 && bytes.get(op_start - 1) == Some(&b' ');
        let space_after = bytes.get(op_end) == Some(&b' ');

        match enforced {
            "space" => {
                if !space_before || !space_after {
                    let (line, column) = source.offset_to_line_col(op_start);
                    return vec![Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Surrounding space missing for operator `=`.".to_string(),
                    }];
                }
            }
            "no_space" => {
                if space_before || space_after {
                    let (line, column) = source.offset_to_line_col(op_start);
                    return vec![Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Surrounding space detected for operator `=`.".to_string(),
                    }];
                }
            }
            _ => {}
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
            &SpaceAroundEqualsInParameterDefault,
            include_bytes!(
                "../../../testdata/cops/layout/space_around_equals_in_parameter_default/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &SpaceAroundEqualsInParameterDefault,
            include_bytes!(
                "../../../testdata/cops/layout/space_around_equals_in_parameter_default/no_offense.rb"
            ),
        );
    }
}
