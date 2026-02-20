use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, NIL_NODE, STRING_NODE, SYMBOL_NODE};

pub struct TimeZone;

impl Cop for TimeZone {
    fn name(&self) -> &'static str {
        "Rails/TimeZone"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE, NIL_NODE, STRING_NODE, SYMBOL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();

        // Methods that are timezone-unsafe on Time (matches RuboCop's DANGEROUS_METHODS)
        // Note: utc, gm, mktime are NOT dangerous — they already produce UTC times
        let is_unsafe_method = matches!(
            method,
            b"now" | b"parse" | b"at" | b"new" | b"local"
        );
        if !is_unsafe_method {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        // Handle both ConstantReadNode (Time) and ConstantPathNode (::Time)
        if util::constant_name(&recv) != Some(b"Time") {
            return;
        }

        // RuboCop skips Time.parse/new/at when the first string argument already has
        // a timezone specifier (e.g., "2023-05-29 00:00:00 UTC", "2015-03-02T19:05:37Z",
        // "2015-03-02T19:05:37+05:00"). Pattern: /([A-Za-z]|[+-]\d{2}:?\d{2})\z/
        if let Some(args) = call.arguments() {
            let first_arg = args.arguments().iter().next();
            if let Some(arg) = first_arg {
                if let Some(str_node) = arg.as_string_node() {
                    let content = str_node.unescaped().as_ref();
                    if has_timezone_specifier(content) {
                        return;
                    }
                }
            }
        }

        // Skip Time.new/at/now with `in:` keyword argument (timezone offset provided)
        if method == b"at" || method == b"now" || method == b"new" {
            if has_in_keyword_arg(&call) {
                return;
            }
        }
        // Time.new with 7 arguments (last is timezone offset)
        if method == b"new" {
            if let Some(args) = call.arguments() {
                let arg_count = args.arguments().iter().count();
                if arg_count == 7 {
                    return;
                }
            }
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
                    return;
                }
            }
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `Time.zone.{}` instead of `Time.{}`.",
                String::from_utf8_lossy(method),
                String::from_utf8_lossy(method)
            ),
        ));
    }
}

/// Check if a call has an `in:` keyword argument (for timezone offset).
fn has_in_keyword_arg(call: &ruby_prism::CallNode<'_>) -> bool {
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };

    // Check the last argument for a keyword hash with `in:` key
    let last_arg = args.arguments().iter().last();
    if let Some(arg) = last_arg {
        // Keyword hash argument (keyword args in method calls)
        if let Some(kw_hash) = arg.as_keyword_hash_node() {
            for elem in kw_hash.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if &*sym.unescaped() == b"in" {
                            // Value must not be nil
                            if assoc.value().as_nil_node().is_none() {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        // Regular hash argument
        if let Some(hash) = arg.as_hash_node() {
            for elem in hash.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(sym) = assoc.key().as_symbol_node() {
                        if &*sym.unescaped() == b"in" {
                            if assoc.value().as_nil_node().is_none() {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check if a string value ends with a timezone specifier.
/// Matches RuboCop's TIMEZONE_SPECIFIER: /([A-Za-z]|[+-]\d{2}:?\d{2})\z/
fn has_timezone_specifier(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let last = bytes[bytes.len() - 1];
    // Ends with a letter (e.g., "UTC", "Z", "EST")
    if last.is_ascii_alphabetic() {
        return true;
    }
    // Ends with +/-HH:MM or +/-HHMM pattern
    // Check for pattern: [+-]\d{2}:?\d{2} at end
    let len = bytes.len();
    // +/-HHMM (5 chars) or +/-HH:MM (6 chars)
    if len >= 6 {
        let s = &bytes[len - 6..];
        if (s[0] == b'+' || s[0] == b'-')
            && s[1].is_ascii_digit()
            && s[2].is_ascii_digit()
            && s[3] == b':'
            && s[4].is_ascii_digit()
            && s[5].is_ascii_digit()
        {
            return true;
        }
    }
    if len >= 5 {
        let s = &bytes[len - 5..];
        if (s[0] == b'+' || s[0] == b'-')
            && s[1].is_ascii_digit()
            && s[2].is_ascii_digit()
            && s[3].is_ascii_digit()
            && s[4].is_ascii_digit()
        {
            return true;
        }
    }
    false
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
                || !after.unwrap().is_ascii_alphanumeric() && after != Some(b'_')
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
