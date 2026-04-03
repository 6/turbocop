use crate::cop::shared::node_type::IF_NODE;
use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Fixes the main Prism parity gaps for this cop:
/// - nitrocop only handled ternary `if` nodes and returned early for keyword
///   `if` and `elsif`, which missed corpus cases like threshold guards and
///   clamp-like helper methods.
/// - parenthesized ternary predicates such as `(a >= b) ? b : a` wrap the
///   comparison in `ParenthesesNode`, so `predicate.as_call_node()` missed
///   RuboCop-covered cases.
/// - The fix keeps RuboCop's shape: only single-expression branches with a
///   final `else` are compared, while the predicate may be parenthesized.
pub struct MinMaxComparison;

impl Cop for MinMaxComparison {
    fn name(&self) -> &'static str {
        "Style/MinMaxComparison"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let if_node = match node.as_if_node() {
            Some(if_node) => if_node,
            None => return,
        };

        let cmp_call = match extract_comparison_call(if_node.predicate()) {
            Some(call) => call,
            None => return,
        };

        let op = cmp_call.name();
        let op_bytes = op.as_slice();
        if op_bytes != b">" && op_bytes != b">=" && op_bytes != b"<" && op_bytes != b"<=" {
            return;
        }

        let cmp_lhs = match cmp_call.receiver() {
            Some(receiver) => receiver,
            None => return,
        };

        let cmp_args = match cmp_call.arguments() {
            Some(arguments) => arguments,
            None => return,
        };
        let cmp_arg_list: Vec<_> = cmp_args.arguments().iter().collect();
        if cmp_arg_list.len() != 1 {
            return;
        }
        let cmp_rhs = &cmp_arg_list[0];

        let cons_expr = match extract_single_stmt(if_node.statements()) {
            Some(expr) => expr,
            None => return,
        };
        let alt_expr = match extract_else_stmt(if_node.subsequent()) {
            Some(expr) => expr,
            None => return,
        };

        let lhs_src = node_source(source, &cmp_lhs);
        let rhs_src = node_source(source, cmp_rhs);
        let cons_src = node_source(source, &cons_expr);
        let alt_src = node_source(source, &alt_expr);

        let suggestion = match op_bytes {
            b">" | b">=" => {
                if lhs_src == cons_src && rhs_src == alt_src {
                    "max"
                } else if lhs_src == alt_src && rhs_src == cons_src {
                    "min"
                } else {
                    return;
                }
            }
            b"<" | b"<=" => {
                if lhs_src == cons_src && rhs_src == alt_src {
                    "min"
                } else if lhs_src == alt_src && rhs_src == cons_src {
                    "max"
                } else {
                    return;
                }
            }
            _ => return,
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `[{lhs_src}, {rhs_src}].{suggestion}` instead."),
        ));
    }
}

fn extract_comparison_call<'a>(node: ruby_prism::Node<'a>) -> Option<ruby_prism::CallNode<'a>> {
    util::unwrap_parentheses(node).as_call_node()
}

fn extract_single_stmt<'a>(
    statements: Option<ruby_prism::StatementsNode<'a>>,
) -> Option<ruby_prism::Node<'a>> {
    let statements = statements?;
    let body = statements.body();
    if body.len() != 1 {
        return None;
    }

    body.iter().next()
}

fn extract_else_stmt<'a>(subsequent: Option<ruby_prism::Node<'a>>) -> Option<ruby_prism::Node<'a>> {
    let else_node = subsequent?.as_else_node()?;
    extract_single_stmt(else_node.statements())
}

fn node_source<'a>(source: &'a SourceFile, node: &ruby_prism::Node<'_>) -> &'a str {
    source.byte_slice(
        node.location().start_offset(),
        node.location().end_offset(),
        "",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MinMaxComparison, "cops/style/min_max_comparison");
}
