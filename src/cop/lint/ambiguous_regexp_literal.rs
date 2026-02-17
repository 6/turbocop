use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AmbiguousRegexpLiteral;

impl Cop for AmbiguousRegexpLiteral {
    fn name(&self) -> &'static str {
        "Lint/AmbiguousRegexpLiteral"
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
        // Look for CallNode without parentheses where the first argument is a
        // RegularExpressionNode or a MatchWriteNode wrapping one.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must not have parentheses
        if call.opening_loc().is_some() {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        if args.is_empty() {
            return Vec::new();
        }

        let first_arg = args.iter().next().unwrap();

        // Check if the first argument is a regexp literal
        let regexp_offset = if first_arg.as_regular_expression_node().is_some() {
            Some(first_arg.location().start_offset())
        } else if let Some(mw) = first_arg.as_match_write_node() {
            // MatchWriteNode wraps a regexp =~ call
            let match_call = mw.call();
            if let Some(recv) = match_call.receiver() {
                if recv.as_regular_expression_node().is_some() {
                    Some(recv.location().start_offset())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let regexp_start = match regexp_offset {
            Some(o) => o,
            None => return Vec::new(),
        };

        // Check there's a space between the method name and the `/`
        // In Prism, when there are no parens, the call node's message_loc ends
        // before the argument. We need to check if there's whitespace between
        // the method name and the `/`.
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };
        let msg_end = msg_loc.end_offset();

        // There must be at least one space between method name end and regexp start
        if regexp_start <= msg_end {
            return Vec::new();
        }

        let between = &source.as_bytes()[msg_end..regexp_start];
        if !between.iter().all(|&b| b == b' ' || b == b'\t') {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(regexp_start);
        vec![self.diagnostic(
            source,
            line,
            column,
            "Ambiguous regexp literal. Parenthesize the method arguments if it's surely a regexp literal, or add a whitespace to the right of the `/` if it should be a division.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AmbiguousRegexpLiteral, "cops/lint/ambiguous_regexp_literal");
}
