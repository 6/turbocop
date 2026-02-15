use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantMatch;

impl Cop for RedundantMatch {
    fn name(&self) -> &'static str {
        "Performance/RedundantMatch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"match" {
            return Vec::new();
        }

        // Must have a receiver (x.match)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Must have arguments (x.match(y))
        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // RuboCop only flags when a string or regexp literal appears on one side.
        // This avoids false positives on e.g. `pattern.match(variable)` where
        // both sides are non-literals.
        let first_arg = match arguments.arguments().iter().next() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let recv_is_literal = receiver.as_string_node().is_some()
            || receiver.as_regular_expression_node().is_some();
        let arg_is_literal = first_arg.as_string_node().is_some()
            || first_arg.as_regular_expression_node().is_some();

        if !recv_is_literal && !arg_is_literal {
            return Vec::new();
        }

        // Don't flag if the call has a block (MatchData is passed to it)
        if call.block().is_some() {
            return Vec::new();
        }

        // Don't flag if the result is chained (e.g., .match(x)[1], .match(x).to_s)
        let call_end = call.location().end_offset();
        let bytes = source.as_bytes();
        let mut pos = call_end;
        while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
            pos += 1;
        }
        if pos < bytes.len() {
            let next = bytes[pos];
            // Result is chained (.method), indexed ([]), safe-navigated (&.),
            // or used as a sub-expression / argument (closing paren)
            if next == b'.' || next == b'[' || next == b'&' || next == b')' {
                return Vec::new();
            }
        }

        // Don't flag if the result is assigned (e.g., m = str.match(x))
        let (_, recv_col) = source.offset_to_line_col(receiver.location().start_offset());
        let call_line_num = source.offset_to_line_col(call.location().start_offset()).0;
        if let Some(line_bytes) = source.lines().nth(call_line_num - 1) {
            let before_recv = &line_bytes[..recv_col.min(line_bytes.len())];
            if before_recv.iter().any(|&b| b == b'=') {
                return Vec::new();
            }
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `match?` instead of `match` when `MatchData` is not used.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantMatch, "cops/performance/redundant_match");
}
