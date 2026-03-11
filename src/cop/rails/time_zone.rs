use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/TimeZone — checks for Time methods without zone.
///
/// ## Investigation (2026-03-10)
///
/// **FP root cause (qualified constant paths):** `util::constant_name()` extracted
/// just the last segment of a ConstantPathNode, so `Some::Time.now` matched as
/// `Time` and was falsely flagged. Fix: inline the constant check to verify the
/// receiver is either a bare `Time` (ConstantReadNode) or root-qualified `::Time`
/// (ConstantPathNode with parent=None/cbase). Matches RuboCop's `(const {nil? cbase} :Time)`.
///
/// **FN root cause (extra SAFE_METHODS):** `getutc`, `rfc2822`, `rfc822`, `to_r`
/// were in the safe methods list but are NOT in RuboCop's ACCEPTED_METHODS, causing
/// `Time.now.getutc` etc. to be incorrectly exempted. Removed these methods.
///
/// **Remaining gaps:**
/// - `localtime` handling: RuboCop treats `localtime` without args as an offense
///   (`MSG_LOCALTIME`) and `localtime` with args as accepted. Nitrocop currently
///   treats all `localtime` as safe (byte scanner limitation). This causes FNs for
///   bare `Time.now.localtime`.
/// - Strict mode does not check GOOD_METHODS chain (e.g., `Time.now.zone` is
///   flagged in strict mode but shouldn't be). Requires AST parent walking.
/// - `String#to_time` detection (RuboCop's `on_send` for `to_time`) not implemented.
/// - Byte-level chain scanner vs RuboCop's AST parent walking: the scanner works
///   correctly for most cases because `call.location().end_offset()` ends at the
///   closing paren of arguments, so `foo(Time.now).utc` correctly sees `)` (not
///   `.utc`) after `Time.now`. Edge cases with complex nesting may still diverge.
pub struct TimeZone;

impl Cop for TimeZone {
    fn name(&self) -> &'static str {
        "Rails/TimeZone"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let is_unsafe_method = matches!(method, b"now" | b"parse" | b"at" | b"new" | b"local");
        if !is_unsafe_method {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        // Handle ConstantReadNode (Time) and ConstantPathNode (::Time) but NOT
        // qualified paths like Some::Time — only unqualified or root-qualified.
        // RuboCop: (const {nil? cbase} :Time)
        if let Some(cr) = recv.as_constant_read_node() {
            if cr.name().as_slice() != b"Time" {
                return;
            }
        } else if let Some(cp) = recv.as_constant_path_node() {
            // ::Time — parent must be None (cbase), not Some::Time
            if cp.parent().is_some() {
                return;
            }
            if cp.name().map(|n| n.as_slice()) != Some(b"Time") {
                return;
            }
        } else {
            return;
        }

        // RuboCop skips Time.parse/new/at when the first string argument already has
        // a timezone specifier (e.g., "2023-05-29 00:00:00 UTC", "2015-03-02T19:05:37Z",
        // "2015-03-02T19:05:37+05:00"). Pattern: /([A-Za-z]|[+-]\d{2}:?\d{2})\z/
        if let Some(args) = call.arguments() {
            let first_arg = args.arguments().iter().next();
            if let Some(arg) = first_arg {
                if let Some(str_node) = arg.as_string_node() {
                    let content = str_node.unescaped();
                    if has_timezone_specifier(content) {
                        return;
                    }
                }
            }
        }

        // Skip Time.new/at/now with `in:` keyword argument (timezone offset provided)
        if (method == b"at" || method == b"now" || method == b"new") && has_in_keyword_arg(&call) {
            return;
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
            // In flexible mode, Time.now (and others) are acceptable if ANY method
            // in the subsequent chain is timezone-safe (e.g., .utc, .in_time_zone).
            // RuboCop walks up the AST via node.parent; we scan forward through the
            // source bytes following the method chain.
            // Example: Time.at(x).to_datetime.in_time_zone(...) — the chain after
            // Time.at(x) is ".to_datetime.in_time_zone(...)" and in_time_zone is safe.
            let bytes = source.as_bytes();
            let end = call.location().end_offset();
            if chain_contains_tz_safe_method(bytes, end) {
                return;
            }
        }

        let loc = call.message_loc().unwrap_or(call.location());
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
                        if sym.unescaped() == b"in" {
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
                        if sym.unescaped() == b"in" && assoc.value().as_nil_node().is_none() {
                            return true;
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

/// Scan forward through a method chain starting at `pos` in `bytes`, returning
/// true if any method in the chain is a timezone-safe method. Handles chains
/// like `.to_datetime.in_time_zone(...)` by following `.method(args)` segments.
fn chain_contains_tz_safe_method(bytes: &[u8], start: usize) -> bool {
    // Matches RuboCop's ACCEPTED_METHODS + GOOD_METHODS + [:current] for flexible mode.
    // Notably excludes getutc, rfc2822, rfc822, to_r which are NOT in RuboCop's lists.
    // localtime is kept because RuboCop accepts localtime WITH arguments (handled specially).
    const SAFE_METHODS: &[&[u8]] = &[
        b"utc",
        b"getlocal",
        b"in_time_zone",
        b"localtime",
        b"iso8601",
        b"xmlschema",
        b"jisx0301",
        b"rfc3339",
        b"httpdate",
        b"to_i",
        b"to_f",
        b"zone",
        b"current",
    ];

    let mut pos = start;
    loop {
        // Skip whitespace (including newlines for multi-line chains)
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }

        // Must see '.' or '&.' to continue the chain
        if pos >= bytes.len() || (bytes[pos] != b'.' && bytes[pos] != b'&') {
            return false;
        }
        if bytes[pos] == b'&' {
            pos += 1;
            if pos >= bytes.len() || bytes[pos] != b'.' {
                return false;
            }
        }
        pos += 1; // skip the '.'

        // Skip whitespace after dot
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }

        // Read the method name
        let method_start = pos;
        while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
            pos += 1;
        }
        if pos == method_start {
            return false; // no method name found
        }
        let method = &bytes[method_start..pos];

        // Check if this method is timezone-safe
        if SAFE_METHODS.contains(&method) {
            return true;
        }

        // Skip past arguments if present: balanced parentheses
        // Skip whitespace first
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos < bytes.len() && bytes[pos] == b'(' {
            let mut depth = 1u32;
            pos += 1;
            while pos < bytes.len() && depth > 0 {
                match bytes[pos] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    b'\'' | b'"' => {
                        // Skip string literals to avoid counting parens inside strings
                        let quote = bytes[pos];
                        pos += 1;
                        while pos < bytes.len() && bytes[pos] != quote {
                            if bytes[pos] == b'\\' {
                                pos += 1; // skip escaped char
                            }
                            pos += 1;
                        }
                        // pos is at closing quote, will be incremented below
                    }
                    _ => {}
                }
                pos += 1;
            }
        }
        // Continue to check next chain element
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TimeZone, "cops/rails/time_zone");
}
