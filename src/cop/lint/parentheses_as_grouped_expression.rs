use crate::cop::node_type::{CALL_NODE, PARENTHESES_NODE, RANGE_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ParenthesesAsGroupedExpression;

impl Cop for ParenthesesAsGroupedExpression {
    fn name(&self) -> &'static str {
        "Lint/ParenthesesAsGroupedExpression"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, PARENTHESES_NODE, RANGE_NODE, STATEMENTS_NODE]
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
        // Look for method calls where there's a space before the opening parenthesis.
        // `a.func (x)` should be `a.func(x)`.
        //
        // Prism parses `a.func (x)` as a call WITHOUT opening_loc (no call parens),
        // where the first argument is a ParenthesesNode wrapping `x`.
        // So we look for CallNode with no opening_loc where:
        // 1. There is a message_loc (method name)
        // 2. The first argument is a ParenthesesNode
        // 3. There's whitespace between message end and the `(`
        // 4. There's only ONE argument (the ParenthesesNode)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must NOT have opening_loc (no call-level parens)
        if call.opening_loc().is_some() {
            return;
        }

        // Skip operator methods (%, +, -, ==, etc.)
        let method_name = call.name().as_slice();
        if is_operator(method_name) {
            return;
        }

        // Must have a method name
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return,
        };

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();

        // Must have exactly one argument (the parenthesized expression)
        if args.len() != 1 {
            return;
        }

        let first_arg = args.iter().next().unwrap();

        // The argument must be a ParenthesesNode
        let paren_node = match first_arg.as_parentheses_node() {
            Some(p) => p,
            None => return,
        };

        // There must be a space between method name end and the `(` of the ParenthesesNode
        let msg_end = msg_loc.end_offset();
        let paren_start = paren_node.location().start_offset();

        if paren_start <= msg_end {
            return;
        }

        let between = &source.as_bytes()[msg_end..paren_start];
        if between.is_empty() || !between.iter().all(|&b| b == b' ' || b == b'\t') {
            return;
        }

        // Check what's after the closing paren - if there's an operator, method chain,
        // ternary, or hash rocket, it's not a grouped expression
        let paren_end = paren_node.location().end_offset();
        let call_end = call.location().end_offset();

        // If the call extends beyond the paren, something follows (operator/chain)
        if call_end > paren_end {
            return;
        }

        // Check what's after the closing paren on the same line
        if paren_end < source.as_bytes().len() {
            let rest = &source.as_bytes()[paren_end..];
            // Find the first non-whitespace character
            let trimmed = rest.iter().position(|&b| b != b' ' && b != b'\t');
            if let Some(pos) = trimmed {
                // Stop at newline
                if rest[pos] != b'\n' && rest[pos] != b'\r' {
                    let ch = rest[pos];
                    // Hash rocket
                    if rest[pos..].starts_with(b"=>") {
                        return;
                    }
                    // Ternary
                    if ch == b'?' {
                        return;
                    }
                    // Binary operators
                    if ch == b'|' || ch == b'&' || ch == b'+' || ch == b'-' || ch == b'*' {
                        return;
                    }
                    // Method chain
                    if ch == b'.' {
                        return;
                    }
                }
            }
        }

        // Check for compound range inside the parens: `rand (a - b)..(c - d)`
        if let Some(body) = paren_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let inner = stmts.body();
                if inner.len() == 1 {
                    let expr = inner.iter().next().unwrap();
                    if let Some(range) = expr.as_range_node() {
                        // Check if sub-expressions are compound (not simple literals)
                        let is_compound = |n: &ruby_prism::Node<'_>| -> bool {
                            n.as_call_node().is_some() || n.as_parentheses_node().is_some()
                        };
                        let left_compound = range.left().map(|l| is_compound(&l)).unwrap_or(false);
                        let right_compound =
                            range.right().map(|r| is_compound(&r)).unwrap_or(false);
                        if left_compound || right_compound {
                            return;
                        }
                    }
                }
            }
        }

        // Build the argument text for the message
        let arg_text = source.byte_slice(paren_start, paren_end, "(...)");

        let (line, column) = source.offset_to_line_col(paren_start);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("`{}` interpreted as grouped expression.", arg_text),
        ));
    }
}

fn is_operator(name: &[u8]) -> bool {
    matches!(
        name,
        b"=="
            | b"!="
            | b"<"
            | b">"
            | b"<="
            | b">="
            | b"<=>"
            | b"+"
            | b"-"
            | b"*"
            | b"/"
            | b"%"
            | b"**"
            | b"&"
            | b"|"
            | b"^"
            | b"~"
            | b"<<"
            | b">>"
            | b"[]"
            | b"[]="
            | b"=~"
            | b"!~"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ParenthesesAsGroupedExpression,
        "cops/lint/parentheses_as_grouped_expression"
    );
}
