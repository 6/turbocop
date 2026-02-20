use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::OPTIONAL_PARAMETER_NODE;

pub struct SpaceAroundEqualsInParameterDefault;

impl Cop for SpaceAroundEqualsInParameterDefault {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundEqualsInParameterDefault"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[OPTIONAL_PARAMETER_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let opt = match node.as_optional_parameter_node() {
            Some(o) => o,
            None => return,
        };

        let enforced = config.get_str("EnforcedStyle", "space");

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
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Surrounding space missing for operator `=`.".to_string(),
                    ));
                }
            }
            "no_space" => {
                if space_before || space_after {
                    let (line, column) = source.offset_to_line_col(op_start);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Surrounding space detected for operator `=`.".to_string(),
                    ));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceAroundEqualsInParameterDefault,
        "cops/layout/space_around_equals_in_parameter_default"
    );
}
