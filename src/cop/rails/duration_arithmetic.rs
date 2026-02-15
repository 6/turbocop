use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DurationArithmetic;

const DURATION_METHODS: &[&[u8]] = &[
    b"second", b"seconds", b"minute", b"minutes",
    b"hour", b"hours", b"day", b"days",
    b"week", b"weeks", b"fortnight", b"fortnights",
    b"month", b"months", b"year", b"years",
];

/// Check if a node matches Time.current or Time.zone.now (or ::Time variants).
/// Note: Time.now is NOT matched — only Time.current and Time.zone.now per RuboCop.
fn is_time_current(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    let method = call.name().as_slice();
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };

    // Pattern 1: Time.current or ::Time.current
    if method == b"current" {
        return util::constant_name(&recv) == Some(b"Time");
    }

    // Pattern 2: Time.zone.now or ::Time.zone.now
    if method == b"now" {
        if let Some(zone_call) = recv.as_call_node() {
            if zone_call.name().as_slice() == b"zone" {
                if let Some(time_recv) = zone_call.receiver() {
                    return util::constant_name(&time_recv) == Some(b"Time");
                }
            }
        }
    }

    false
}

/// Check if a node is a duration method call (e.g., 1.day, 2.5.weeks)
fn is_duration(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if !DURATION_METHODS.contains(&call.name().as_slice()) {
        return false;
    }

    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };

    // Receiver must be int, float, or a method call (e.g., a variable)
    recv.as_integer_node().is_some()
        || recv.as_float_node().is_some()
        || (recv.as_call_node().is_some() && recv.as_call_node().unwrap().receiver().is_none())
}

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

        // Receiver should be Time.current or Time.zone.now
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        if !is_time_current(&recv) {
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

        if !is_duration(&arg_list[0]) {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not add or subtract duration.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DurationArithmetic, "cops/rails/duration_arithmetic");
}
