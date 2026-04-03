use crate::cop::shared::node_type::{INTERPOLATED_X_STRING_NODE, X_STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus FN investigation (0 FP / 11 FN): RuboCop treats any backtick byte inside
/// a backtick command body's source as a reason to prefer `%x`, including nested
/// command literals inside interpolation such as ``#{`status`}`` and multiline
/// callbacks that call ``block.call(`realpath`)``. The previous implementation only
/// looked for the escaped `\\`` sequence in backtick literals and, under
/// `EnforcedStyle: backticks`, never emitted the `%x` offense for backtick literals
/// whose bodies already contained backticks. Match RuboCop by scanning the literal
/// body between the opening and closing delimiters for any backtick byte and by
/// flagging those backtick literals as `%x`.
pub struct CommandLiteral;

impl Cop for CommandLiteral {
    fn name(&self) -> &'static str {
        "Style/CommandLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTERPOLATED_X_STRING_NODE, X_STRING_NODE]
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
        let enforced_style = config.get_str("EnforcedStyle", "backticks");
        let allow_inner_backticks = config.get_bool("AllowInnerBackticks", false);

        // Check both XStringNode and InterpolatedXStringNode
        let (opening, closing, node_loc, node_source) = if let Some(x) = node.as_x_string_node() {
            (
                x.opening_loc(),
                x.closing_loc(),
                x.location(),
                x.location().as_slice().to_vec(),
            )
        } else if let Some(x) = node.as_interpolated_x_string_node() {
            (
                x.opening_loc(),
                x.closing_loc(),
                x.location(),
                x.location().as_slice().to_vec(),
            )
        } else {
            return;
        };

        let opening_bytes = opening.as_slice();
        let body = source
            .as_bytes()
            .get(opening.end_offset()..closing.start_offset())
            .unwrap_or(&[]);
        let is_backtick = opening_bytes == b"`";
        let is_multiline = node_source.iter().filter(|&&b| b == b'\n').count() > 1;
        let content_has_backticks = body.contains(&b'`');

        let disallowed_backtick = !allow_inner_backticks && content_has_backticks;

        match enforced_style {
            "backticks" => {
                if is_backtick && disallowed_backtick {
                    let (line, column) = source.offset_to_line_col(node_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `%x` around command string.".to_string(),
                    ));
                } else if !is_backtick && !disallowed_backtick {
                    // Flag %x usage unless it contains backticks (and AllowInnerBackticks is false)
                    let (line, column) = source.offset_to_line_col(node_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use backticks around command string.".to_string(),
                    ));
                }
            }
            "percent_x" => {
                // Flag backtick usage
                if is_backtick {
                    let (line, column) = source.offset_to_line_col(node_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `%x` around command string.".to_string(),
                    ));
                }
            }
            "mixed" => {
                if is_backtick && (is_multiline || disallowed_backtick) {
                    let (line, column) = source.offset_to_line_col(node_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `%x` around command string.".to_string(),
                    ));
                } else if !is_backtick && !is_multiline && !disallowed_backtick {
                    let (line, column) = source.offset_to_line_col(node_loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use backticks around command string.".to_string(),
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
    crate::cop_fixture_tests!(CommandLiteral, "cops/style/command_literal");
}
