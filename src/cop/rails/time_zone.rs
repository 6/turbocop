use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TimeZone;

impl Cop for TimeZone {
    fn name(&self) -> &'static str {
        "Rails/TimeZone"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = call.name().as_slice();

        // Methods that are timezone-unsafe on Time
        let is_unsafe_method = matches!(
            method,
            b"now" | b"parse" | b"at" | b"new" | b"mktime" | b"local" | b"gm" | b"utc"
        );
        if !is_unsafe_method {
            return Vec::new();
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let const_read = match recv.as_constant_read_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if const_read.name().as_slice() != b"Time" {
            return Vec::new();
        }

        let style = config.get_str("EnforcedStyle", "flexible");

        if style == "flexible" {
            // In flexible mode, Time.now (and others) are acceptable if followed
            // by a timezone-aware method like .utc, .in_time_zone, .getutc, etc.
            let bytes = source.as_bytes();
            let end = call.location().end_offset();
            if end < bytes.len() && bytes[end] == b'.' {
                // Check if a timezone-safe method follows
                let rest = &bytes[end + 1..];
                if starts_with_tz_safe_method(rest) {
                    return Vec::new();
                }
            }
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `Time.zone.{}` instead of `Time.{}`.",
                String::from_utf8_lossy(method),
                String::from_utf8_lossy(method)
            ),
        )]
    }
}

/// Check if the bytes start with a timezone-safe method name followed by a
/// non-identifier character (or end of file).
fn starts_with_tz_safe_method(bytes: &[u8]) -> bool {
    const SAFE_METHODS: &[&[u8]] = &[
        b"utc",
        b"getutc",
        b"getlocal",
        b"in_time_zone",
        b"localtime",
        b"iso8601",
        b"xmlschema",
        b"httpdate",
        b"rfc2822",
        b"rfc822",
        b"to_i",
        b"to_f",
        b"to_r",
    ];
    for method in SAFE_METHODS {
        if bytes.starts_with(method) {
            let after = bytes.get(method.len()).copied();
            // Must be followed by non-identifier char or EOF
            if after.is_none()
                || matches!(after, Some(b'(' | b' ' | b'\n' | b'\r' | b'\t' | b'.' | b','))
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TimeZone, "cops/rails/time_zone");
}
