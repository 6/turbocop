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
/// - Strict mode does not check GOOD_METHODS chain (e.g., `Time.now.zone` is
///   flagged in strict mode but shouldn't be). Requires AST parent walking.
/// - `String#to_time` detection (RuboCop's `on_send` for `to_time`) not implemented.
/// - Byte-level chain scanner vs RuboCop's AST parent walking: the scanner works
///   correctly for most cases because `call.location().end_offset()` ends at the
///   closing paren of arguments, so `foo(Time.now).utc` correctly sees `)` (not
///   `.utc`) after `Time.now`. Edge cases with complex nesting may still diverge.
///
/// ## Investigation (2026-03-15): FP=17, FN=82
///
/// Two fixes:
///
/// **1. `.localtime` without args now treated as unsafe (FN fix, ~82 FNs):**
/// RuboCop treats `.localtime` without arguments as an offense (MSG_LOCALTIME) and
/// `.localtime(offset)` as accepted. Previously all `.localtime` was in SAFE_METHODS.
/// Fix: removed `localtime` from `chain_contains_tz_safe_method` SAFE_METHODS and added
/// special handling that only treats it as safe when followed by `(` with arguments.
///
/// **2. `Time.now` inside `Time.at(..., in:)` no longer flagged (FP fix, ~10 FPs):**
/// `Time.at(Time.now, in: 'UTC')` — the inner `Time.now` was flagged because
/// `enclosing_call_is_safe` only checked SAFE_METHODS (utc, in_time_zone, etc.), not
/// dangerous methods that become safe via `in:` keyword. Fix: added `IN_KEYWORD_METHODS`
/// list (at, new, now) and `enclosing_parens_have_in_keyword` byte scanner.
///
/// ## Investigation (2026-03-14): FP=25
///
/// Two root causes addressed:
///
/// **1. Interpolated string timezone specifier (ManageIQ/feedjira, ~4 FPs):**
/// `Time.parse("#{ts} UTC", ...)` — the first argument is a dstr (interpolated string)
/// ending with a timezone indicator. RuboCop's `attach_timezone_specifier?` checks
/// `date.respond_to?(:value)`. In RuboCop's AST, dstr nodes for `"#{expr} UTC"` have
/// a last child of `str(" UTC")`. The check implicitly covers this via the last part.
/// Fix: added explicit check of the last string literal part of InterpolatedStringNode.
///
/// **2. Time.now inside Time.utc(...) arguments (ice_cube, ~4 FPs):**
/// `Time.utc(Time.now.year - 1, ...)` — RuboCop's parent-chain walking traverses
/// through the argument position into the enclosing call, making chain = [now, year, -, utc].
/// `utc` is in ACCEPTED_METHODS → not_danger_chain? returns true → no offense.
/// Nitrocop's forward byte scanner stops at the `)` following `Time.now.year-1` and
/// doesn't see the outer `Time.utc(...)` call.
/// Fix: added `enclosing_call_is_safe()` backward scan: if Time.now is directly preceded
/// by `safe_method(`, suppress the offense.
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
        // Also handles interpolated strings like "#{ts} UTC" by checking the last
        // string literal part (RuboCop's `dstr.value` implicitly returns last str part).
        if let Some(args) = call.arguments() {
            let first_arg = args.arguments().iter().next();
            if let Some(arg) = first_arg {
                if let Some(str_node) = arg.as_string_node() {
                    let content = str_node.unescaped();
                    if has_timezone_specifier(content) {
                        return;
                    }
                }
                // Handle interpolated strings: check the last literal string part.
                // `"#{ts} UTC"` has last part " UTC" which ends with a letter → safe.
                if let Some(dstr) = arg.as_interpolated_string_node() {
                    let last_str = dstr
                        .parts()
                        .iter()
                        .filter_map(|p| p.as_string_node())
                        .last();
                    if let Some(last) = last_str {
                        if has_timezone_specifier(last.unescaped()) {
                            return;
                        }
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

            // RuboCop also walks UP via node.parent, which means it considers the
            // enclosing call context. For `Time.utc(Time.now.year - 1, ...)`, the
            // chain becomes [now, year, -, utc] and `utc` makes it safe.
            //
            // Detect this by checking if `Time.now` is an immediate argument to a
            // safe method: scan backwards from Time.now's start for `safe_method(`.
            let start = call.location().start_offset();
            if enclosing_call_is_safe(bytes, start) {
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

/// Check if the byte at `start` (beginning of `Time.now` etc.) is immediately
/// inside the argument list of a timezone-safe method call.
///
/// This handles the case where RuboCop's parent-chain walking finds a safe method
/// in the enclosing context. For `Time.utc(Time.now.year - 1, ...)`:
/// - Walking backwards from `Time.now` finds `(` preceded by `utc`
/// - `utc` is in the safe methods list → suppress offense
///
/// This matches RuboCop's behavior where `not_danger_chain?` returns true when
/// the parent-chain (now, year, -, utc) includes an ACCEPTED_METHOD.
fn enclosing_call_is_safe(bytes: &[u8], start: usize) -> bool {
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

    // Methods that become safe when called with `in:` keyword argument
    const IN_KEYWORD_METHODS: &[&[u8]] = &[b"at", b"new", b"now"];

    if start == 0 {
        return false;
    }
    let mut i = start.saturating_sub(1);

    // Skip whitespace before Time.now
    while i > 0 && bytes[i].is_ascii_whitespace() {
        i -= 1;
    }

    // Must be directly preceded by `(` (argument position)
    if bytes[i] != b'(' {
        return false;
    }
    let paren_pos = i;
    if i == 0 {
        return false;
    }
    i -= 1;

    // Skip whitespace before `(`
    while i > 0 && bytes[i].is_ascii_whitespace() {
        i -= 1;
    }

    // Read method name backwards (alphanumeric + underscore + ? + !)
    let end_of_method = i;
    while i > 0
        && (bytes[i].is_ascii_alphanumeric()
            || bytes[i] == b'_'
            || bytes[i] == b'?'
            || bytes[i] == b'!')
    {
        i -= 1;
    }
    // Adjust for the loop decrement
    let method_start = if bytes[i].is_ascii_alphanumeric()
        || bytes[i] == b'_'
        || bytes[i] == b'?'
        || bytes[i] == b'!'
    {
        i
    } else {
        i + 1
    };
    let method_name = &bytes[method_start..=end_of_method];

    if SAFE_METHODS.contains(&method_name) {
        return true;
    }

    // For Time.at/new/now, check if the enclosing call has `in:` keyword argument.
    // E.g., `Time.at(Time.now, in: 'UTC')` — the `in:` makes the outer call safe.
    if IN_KEYWORD_METHODS.contains(&method_name)
        && enclosing_parens_have_in_keyword(bytes, paren_pos)
    {
        return true;
    }

    false
}

/// Check if the parenthesized argument list starting at `paren_pos` contains
/// an `in:` keyword argument. Scans forward from the opening `(` looking for
/// the pattern `in:` preceded by a non-identifier character.
fn enclosing_parens_have_in_keyword(bytes: &[u8], paren_pos: usize) -> bool {
    let mut pos = paren_pos + 1; // skip '('
    let mut depth = 1u32;

    while pos < bytes.len() && depth > 0 {
        match bytes[pos] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return false;
                }
            }
            b'\'' | b'"' => {
                // Skip string literals
                let quote = bytes[pos];
                pos += 1;
                while pos < bytes.len() && bytes[pos] != quote {
                    if bytes[pos] == b'\\' {
                        pos += 1;
                    }
                    pos += 1;
                }
            }
            b'i' if depth == 1 => {
                // Check for `in:` pattern (keyword argument)
                if pos + 2 < bytes.len() && bytes[pos + 1] == b'n' && bytes[pos + 2] == b':' {
                    // Verify it's not part of a longer identifier
                    let before_ok = pos == 0
                        || (!bytes[pos - 1].is_ascii_alphanumeric() && bytes[pos - 1] != b'_');
                    if before_ok {
                        return true;
                    }
                }
            }
            _ => {}
        }
        pos += 1;
    }
    false
}

/// Scan forward through a method chain starting at `pos` in `bytes`, returning
/// true if any method in the chain is a timezone-safe method. Handles chains
/// like `.to_datetime.in_time_zone(...)` by following `.method(args)` segments.
fn chain_contains_tz_safe_method(bytes: &[u8], start: usize) -> bool {
    // Matches RuboCop's ACCEPTED_METHODS + GOOD_METHODS + [:current] for flexible mode.
    // Notably excludes getutc, rfc2822, rfc822, to_r which are NOT in RuboCop's lists.
    // `localtime` is handled specially below: only safe WITH arguments.
    const SAFE_METHODS: &[&[u8]] = &[
        b"utc",
        b"getlocal",
        b"in_time_zone",
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

        // Skip past arguments if present: balanced parentheses, track if args exist
        // Skip whitespace first
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let has_args = if pos < bytes.len() && bytes[pos] == b'(' {
            let mut depth = 1u32;
            pos += 1;
            // Skip whitespace after opening paren
            let mut content_start = pos;
            while content_start < bytes.len() && bytes[content_start].is_ascii_whitespace() {
                content_start += 1;
            }
            // If we immediately hit ')', there are no arguments
            let has_content = content_start < bytes.len() && bytes[content_start] != b')';
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
            has_content
        } else {
            false
        };

        // Check if this method is timezone-safe
        if SAFE_METHODS.contains(&method) {
            return true;
        }
        // `localtime` is only safe when called WITH arguments (timezone offset).
        // Without arguments, it converts to local system time — not timezone-safe.
        if method == b"localtime" && has_args {
            return true;
        }

        // Continue to check next chain element
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TimeZone, "cops/rails/time_zone");
}
