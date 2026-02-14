use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DurationArithmetic;

const TIME_METHODS: &[&[u8]] = &[b"now", b"current"];
const DURATION_METHODS: &[&[u8]] = &[
    b"second", b"seconds", b"minute", b"minutes",
    b"hour", b"hours", b"day", b"days",
    b"week", b"weeks", b"month", b"months",
    b"year", b"years",
];

impl Cop for DurationArithmetic {
    fn name(&self) -> &'static str {
        "Rails/DurationArithmetic"
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

        let method_name = call.name().as_slice();
        if method_name != b"+" && method_name != b"-" {
            return Vec::new();
        }

        // Receiver should be Time.now or Time.current
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let recv_call = match recv.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if !TIME_METHODS.contains(&recv_call.name().as_slice()) {
            return Vec::new();
        }
        let time_recv = match recv_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let const_read = match time_recv.as_constant_read_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if const_read.name().as_slice() != b"Time" {
            return Vec::new();
        }

        // Argument should be a duration method call (e.g., 1.day)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let arg_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if !DURATION_METHODS.contains(&arg_call.name().as_slice()) {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `1.day.from_now` instead of `Time.now + 1.day`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DurationArithmetic, "cops/rails/duration_arithmetic");
}
