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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for method calls where there's a space before the opening parenthesis
        // e.g. `a.func (x)` should be `a.func(x)`
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have parentheses on the arguments
        let open_loc = match call.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        // Must have a method name (message_loc)
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let msg_end = msg_loc.end_offset();
        let open_start = open_loc.start_offset();

        // There must be a space between method name and opening paren
        if open_start <= msg_end {
            return Vec::new();
        }

        let between = &source.as_bytes()[msg_end..open_start];
        if between.is_empty() || !between.iter().all(|&b| b == b' ' || b == b'\t') {
            return Vec::new();
        }

        // Must have arguments
        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();

        // If there are multiple arguments (not wrapped in extra parens), it's not
        // a grouped expression - e.g. `assert_equal (0..1.9), acceleration.domain`
        if args.len() > 1 {
            return Vec::new();
        }

        if args.is_empty() {
            return Vec::new();
        }

        let first_arg = args.iter().next().unwrap();

        // If the single argument is followed by an operator or method chain
        // (the expression continues beyond the closing paren), skip it.
        // e.g. `func (x) || y`, `func (x).foo.bar`, `puts (2 + 3) * 4`
        let close_loc = match call.closing_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let close_end = close_loc.end_offset();
        let call_end = call.location().end_offset();

        // If the call extends beyond the closing paren, there's something after
        // like an operator or method chain
        if call_end > close_end {
            return Vec::new();
        }

        // Check if the argument itself is followed by operators/ternary outside paren.
        // This is tricky; check if the parent call's args extend beyond our close paren.
        // Actually, if the call_end == close_end, the call IS just `method (args)`.

        // Check for hash rocket pattern: `transition (foo - bar) => value`
        // The source after closing paren starts with ` =>`
        if close_end < source.as_bytes().len() {
            let rest = &source.as_bytes()[close_end..];
            let trimmed = rest.iter().position(|&b| b != b' ' && b != b'\t');
            if let Some(pos) = trimmed {
                if rest[pos..].starts_with(b"=>") {
                    return Vec::new();
                }
                // Ternary: `? ...`
                if rest[pos] == b'?' {
                    return Vec::new();
                }
                // Binary operators: ||, &&, +, -, *, etc
                if rest[pos] == b'|' || rest[pos] == b'&' || rest[pos] == b'+' || rest[pos] == b'-' || rest[pos] == b'*' {
                    return Vec::new();
                }
                // Method chain: `.something` or `&.something`
                if rest[pos] == b'.' {
                    return Vec::new();
                }
            }
        }

        // Check for compound range: `rand (a - b)..(c - d)` — skip if content has `..`
        // and the sub-expressions are not simple literals
        if let Some(paren) = first_arg.as_parentheses_node() {
            if let Some(body) = paren.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let inner = stmts.body();
                    if inner.len() == 1 {
                        let expr = inner.iter().next().unwrap();
                        if expr.as_range_node().is_some() {
                            // Check if sub-expressions of the range contain calls/operations
                            // (compound range). Simple literal ranges should still be flagged.
                            if let Some(range) = expr.as_range_node() {
                                let is_compound = |n: &ruby_prism::Node<'_>| -> bool {
                                    n.as_call_node().is_some() || n.as_parentheses_node().is_some()
                                };
                                let left_compound = range.left().map(|l| is_compound(&l)).unwrap_or(false);
                                let right_compound = range.right().map(|r| is_compound(&r)).unwrap_or(false);
                                if left_compound || right_compound {
                                    return Vec::new();
                                }
                            }
                        }
                    }
                }
            }
        }

        // Build the argument text for the message
        let arg_start = open_loc.start_offset();
        let arg_end = close_end;
        let arg_text = std::str::from_utf8(&source.as_bytes()[arg_start..arg_end]).unwrap_or("(...)");

        let (line, column) = source.offset_to_line_col(open_start);
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("`{}` interpreted as grouped expression.", arg_text),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ParenthesesAsGroupedExpression, "cops/lint/parentheses_as_grouped_expression");
}
