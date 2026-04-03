use crate::cop::shared::node_type::EMBEDDED_STATEMENTS_NODE;
use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Prism exposes every `#{...}` body as an `EmbeddedStatementsNode`, including
/// double-quoted strings, backticks, regexps, and symbols. The previous port
/// only walked `InterpolatedStringNode` parts and reported the `#{` opener,
/// which missed non-string interpolation forms and produced line-shifted
/// diagnostics for multiline interpolations.
pub struct EmptyStringInsideInterpolation;

impl Cop for EmptyStringInsideInterpolation {
    fn name(&self) -> &'static str {
        "Style/EmptyStringInsideInterpolation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[EMBEDDED_STATEMENTS_NODE]
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

        let embedded = if let Some(node) = node.as_embedded_statements_node() {
            node
        } else {
            return;
        };

        let Some(statements) = embedded.statements() else {
            return;
        };
        let stmt_list: Vec<_> = statements.body().iter().collect();
        if stmt_list.len() != 1 {
            return;
        }

        match enforced_style {
            "trailing_conditional" => {
                if let Some(if_node) = stmt_list[0].as_if_node() {
                    if branch_is_empty(if_node.statements())
                        || else_branch_is_empty(if_node.subsequent())
                    {
                        add_diagnostic(self, source, &stmt_list[0], diagnostics, MSG_TERNARY);
                    }
                } else if let Some(unless_node) = stmt_list[0].as_unless_node() {
                    if branch_is_empty(unless_node.statements())
                        || branch_is_empty(
                            unless_node.else_clause().and_then(|node| node.statements()),
                        )
                    {
                        add_diagnostic(self, source, &stmt_list[0], diagnostics, MSG_TERNARY);
                    }
                }
            }
            "ternary" => {
                if let Some(if_node) = stmt_list[0].as_if_node() {
                    if util::is_modifier_if(&if_node) {
                        add_diagnostic(
                            self,
                            source,
                            &stmt_list[0],
                            diagnostics,
                            MSG_TRAILING_CONDITIONAL,
                        );
                    }
                } else if let Some(unless_node) = stmt_list[0].as_unless_node() {
                    if util::is_modifier_unless(&unless_node) {
                        add_diagnostic(
                            self,
                            source,
                            &stmt_list[0],
                            diagnostics,
                            MSG_TRAILING_CONDITIONAL,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

const MSG_TRAILING_CONDITIONAL: &str = "Do not use trailing conditionals in string interpolation.";
const MSG_TERNARY: &str = "Do not return empty strings in string interpolation.";

fn add_diagnostic(
    cop: &EmptyStringInsideInterpolation,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    message: &str,
) {
    let loc = node.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(source, line, column, message.to_string()));
}

fn branch_is_empty(branch: Option<ruby_prism::StatementsNode<'_>>) -> bool {
    let Some(statements) = branch else {
        return false;
    };

    let body: Vec<_> = statements.body().iter().collect();
    body.len() == 1 && is_empty_string_or_nil(&body[0])
}

fn else_branch_is_empty(branch: Option<ruby_prism::Node<'_>>) -> bool {
    branch
        .and_then(|node| node.as_else_node())
        .is_some_and(|else_node| branch_is_empty(else_node.statements()))
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        EmptyStringInsideInterpolation,
        "cops/style/empty_string_inside_interpolation"
    );
}
