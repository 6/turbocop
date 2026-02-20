use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ELSE_NODE, EMBEDDED_STATEMENTS_NODE, IF_NODE, INTERPOLATED_STRING_NODE, NIL_NODE, STRING_NODE, UNLESS_NODE};

pub struct EmptyStringInsideInterpolation;

impl Cop for EmptyStringInsideInterpolation {
    fn name(&self) -> &'static str {
        "Style/EmptyStringInsideInterpolation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ELSE_NODE, EMBEDDED_STATEMENTS_NODE, IF_NODE, INTERPOLATED_STRING_NODE, NIL_NODE, STRING_NODE, UNLESS_NODE]
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
        let enforced_style = config.get_str("EnforcedStyle", "trailing_conditional");

        // Look for interpolated strings containing ternaries with empty string branches
        let interp_string = if let Some(n) = node.as_interpolated_string_node() {
            n
        } else {
            return;
        };


        for part in interp_string.parts().iter() {
            if let Some(embedded) = part.as_embedded_statements_node() {
                if let Some(stmts) = embedded.statements() {
                    let stmt_list: Vec<_> = stmts.body().iter().collect();
                    if stmt_list.len() != 1 {
                        continue;
                    }

                    match enforced_style {
                        "trailing_conditional" => {
                            // Check for ternary with empty string as one branch
                            if let Some(ternary) = stmt_list[0].as_if_node() {
                                let has_if_empty = is_empty_value(&ternary.predicate());
                                let _ = has_if_empty; // We need to check the branches

                                let if_body = ternary.statements();
                                let else_body = ternary.subsequent();

                                let if_is_empty = if let Some(body) = if_body {
                                    let stmts: Vec<_> = body.body().iter().collect();
                                    stmts.len() == 1 && is_empty_string_or_nil(&stmts[0])
                                } else {
                                    false
                                };

                                let else_is_empty = if let Some(else_node) = else_body {
                                    if let Some(else_actual) = else_node.as_else_node() {
                                        if let Some(else_stmts) = else_actual.statements() {
                                            let stmts: Vec<_> = else_stmts.body().iter().collect();
                                            stmts.len() == 1 && is_empty_string_or_nil(&stmts[0])
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                };

                                if if_is_empty || else_is_empty {
                                    let loc = embedded.location();
                                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        line,
                                        column,
                                        "Do not return empty strings in string interpolation.".to_string(),
                                    ));
                                }
                            }
                        }
                        "ternary" => {
                            // Check for trailing if/unless in interpolation
                            if let Some(if_mod) = stmt_list[0].as_if_node() {
                                // Check if this is a modifier if (no else, single branch)
                                if if_mod.subsequent().is_none() {
                                    let loc = embedded.location();
                                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        line,
                                        column,
                                        "Do not use trailing conditionals in string interpolation.".to_string(),
                                    ));
                                }
                            }
                            if let Some(_unless_mod) = stmt_list[0].as_unless_node() {
                                let loc = embedded.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Do not use trailing conditionals in string interpolation.".to_string(),
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

    }
}

fn is_empty_string_or_nil(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_nil_node().is_some() {
        return true;
    }
    if let Some(string_node) = node.as_string_node() {
        return string_node.content_loc().as_slice().is_empty();
    }
    false
}

fn is_empty_value(_node: &ruby_prism::Node<'_>) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        EmptyStringInsideInterpolation,
        "cops/style/empty_string_inside_interpolation"
    );
}
