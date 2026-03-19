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
/// - Byte-level chain scanner vs RuboCop's AST parent walking: the scanner works
///   correctly for most cases because `call.location().end_offset()` ends at the
///   closing paren of arguments, so `foo(Time.now).utc` correctly sees `)` (not
///   `.utc`) after `Time.now`. Edge cases with complex nesting may still diverge.
///
/// ## Investigation (2026-03-15): FP=7, FN=59
///
/// Two fixes:
///
/// **1. Nested `Time.now`/`Time.local` inside outer call with safe chain (FP fix, 7 FPs):**
/// `Time.to_mongo(Time.local(...)).zone` — the inner `Time.local(...)` was flagged because
/// `enclosing_call_is_safe` only checked whether the immediate enclosing method was safe
/// (e.g., `to_mongo` — not safe) but didn't scan the chain AFTER the outer call's closing
/// `)`. Also, it only checked the first argument position (`(`), not later arguments
/// (`Time.parse(x, Time.now).iso8601`). Fix: replaced direct `(` check with
/// `find_enclosing_open_paren()` that scans backward through balanced parens to find the
/// containing `(` regardless of argument position. Added `find_matching_close_paren()` to
/// locate the outer call's closing `)`, then `chain_contains_tz_safe_method()` checks the
/// chain continuing after it.
///
/// **2. `String#to_time` detection in both modes (FN fix):**
/// RuboCop's `on_send` fires on `.to_time` unconditionally (no mode check), but ONLY when
/// the receiver is a string literal (`node.receiver&.str_type?`). Previously nitrocop
/// only flagged in strict mode AND flagged variable receivers too. Fixed to:
/// - Fire in both flexible and strict mode (matching RuboCop's unconditional `on_send`)
/// - Only flag when receiver is a string literal node (not variables)
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
/// **2. `Time.now` inside `Time.at(..., in:)` — REVERTED (was FP fix, ~10 FPs):**
/// Previously added `IN_KEYWORD_METHODS` check in `enclosing_call_is_safe_recursive` to
/// suppress inner `Time.now` when the outer call had `in:` keyword. This was incorrect:
/// RuboCop's `in:` keyword only makes the OUTER `Time.at` call safe (handled at line 250
/// via `has_in_keyword_arg`), but inner `Time.now` arguments should still be flagged.
/// The original "FP" was actually correct behavior — removed the IN_KEYWORD_METHODS check
/// and `enclosing_parens_have_in_keyword` function. Fixes 26 FN in corpus.
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
///
/// ## Investigation (2026-03-18): FP=1, FN=361
///
/// **FN root cause (`String#to_time` in flexible mode):**
/// RuboCop's `on_send` fires on `.to_time` in BOTH strict and flexible mode, but only
/// when the receiver is a string literal (`node.receiver&.str_type?`). Nitrocop was only
/// firing in strict mode AND was incorrectly flagging variable receivers (e.g.,
/// `date_str.to_time`). Fixed by: (1) removing the strict-mode gate so `to_time` fires
/// in both modes, (2) checking that the receiver is a string literal node before flagging.
///
/// ## Investigation (2026-03-18): FP=1, FN=334
///
/// **FN root cause (grouping parens treated as method-call parens):**
/// `enclosing_call_is_safe()` found the enclosing `(` via `find_enclosing_open_paren()`,
/// then extracted the "method name" before it. For grouping parens like
/// `(Time.now - 1.day).to_i`, the `(` had no method name — it picked up an identifier
/// from a completely different line/statement. Even when no identifier was found (e.g.,
/// after `=` or `||`), it fell through to check the chain after the closing `)`, where
/// `.to_i` was in SAFE_METHODS and incorrectly suppressed the offense.
///
/// Fix: three guards before treating the enclosing `(` as method-call parens:
/// 1. `method_start > end_of_method` — no identifier at all (preceded by operator/punct)
/// 2. Newline between method name and `(` — identifier is from a different statement
/// 3. Method name is a Ruby keyword (`return`, `if`, etc.) — keywords use grouping parens
///
/// Fixes ~300+ of the 334 FN (grouping paren patterns in corpus).
///
/// ## Investigation (2026-03-18): FP=1, FN=43
///
/// **FN root cause (method name truncation with `?`/`!` suffixes):**
/// `chain_contains_tz_safe_method()` read method names using only alphanumeric + underscore,
/// truncating `utc?` to `utc` which matched SAFE_METHODS and suppressed the offense.
/// Fix: include `?` and `!` in the method name character set so `utc?` is read as `utc?`
/// and does NOT match `utc` in SAFE_METHODS. Fixes 43 FN across jruby, natalie-lang,
/// sidekiq and other corpus repos.
///
/// **FP root cause (deeply nested parens):**
/// `Time.parse(helper_method(Time.now)).utc` — the inner `Time.now` is nested two levels
/// deep. `enclosing_call_is_safe()` only checked the immediate enclosing `(` (from
/// `helper_method`), which was not safe, and the chain after `helper_method(...)` was `)`
/// (not `.utc`). Fix: made `enclosing_call_is_safe()` recursive (up to 3 levels) so it
/// checks the next enclosing `(` (from `Time.parse`), whose chain after `)` is `.utc`.
///
/// ## Investigation (2026-03-19): FN=16
///
/// Two fixes:
///
/// **1. Grouping parens with space (12 FN):** `schedule (Time.now - 60).to_f` — backward
/// scan found `schedule` before `(`, but the space between `schedule` and `(` means the
/// `(` is a subexpression grouping paren. The `.to_f` after `)` chains on the grouped
/// expression, not the `schedule` call. Fix: detect space/tab in the gap between method
/// name and `(`; when present, skip the chain-after-closing-paren check.
///
/// **2. Safe navigation `&.` chain break (2+ FN):** `Time.at(val)&.utc` — in RuboCop,
/// `csend` (safe navigation) is not `send_type?`, so `extract_method_chain` stops at
/// `&.utc`. The chain is just `[:at]` — `utc` is never reached. Fix: removed `&.` handling
/// from `chain_contains_tz_safe_method`; only regular `.` continues the chain. Since
/// whitespace is skipped first, `&.utc` lands on `&` (not `.`) and correctly breaks.
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

        // String#to_time detection — fires in BOTH strict and flexible mode.
        // RuboCop's on_send fires unconditionally (no mode check) but only when the
        // receiver is a string literal (`node.receiver&.str_type?`). This means
        // `"string".to_time` is always an offense, but `variable.to_time` is not.
        // Also skips when the string has a timezone specifier (e.g., "...Z").
        if method == b"to_time" {
            // Only flag when receiver is a string literal
            if let Some(recv) = call.receiver() {
                if let Some(str_node) = recv.as_string_node() {
                    let content = str_node.unescaped();
                    if !has_timezone_specifier(content) {
                        let loc = call.message_loc().unwrap_or(call.location());
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(
                            self.diagnostic(
                                source,
                                line,
                                column,
                                "Do not use `String#to_time` without zone. Use `Time.zone.parse` instead."
                                    .to_string(),
                            ),
                        );
                    }
                }
                // Non-string receivers (variables, expressions) are never flagged
            }
            // No receiver (bare `to_time`) — not an offense
            return;
        }

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
    // Check up to 3 levels of nesting to handle cases like:
    // Time.parse(helper_method(Time.now)).utc
    // Level 1: helper_method( — not safe, chain after ) is ) — not safe
    // Level 2: Time.parse( — not safe itself, but chain after ) is .utc — safe!
    enclosing_call_is_safe_recursive(bytes, start, 3)
}

fn enclosing_call_is_safe_recursive(bytes: &[u8], start: usize, max_depth: u8) -> bool {
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

    if start == 0 || max_depth == 0 {
        return false;
    }

    // Find the opening `(` of the enclosing call by scanning backward.
    // Time.now may be the first argument (preceded by `(`) or a later argument
    // (preceded by `, ` or similar). We scan backward, tracking parenthesis depth,
    // to find the matching opening `(`.
    let paren_pos = match find_enclosing_open_paren(bytes, start) {
        Some(p) => p,
        None => return false,
    };

    if paren_pos == 0 {
        return false;
    }
    let mut i = paren_pos - 1;

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

    // Check if the `(` is actually a method-call argument paren vs a grouping paren.
    // Grouping parens: `(Time.now - 1.day).to_i`, `return (Time.now).to_i`
    // Method-call parens: `foo(Time.now)`, `Time.utc(Time.now)`
    //
    // Two checks:
    // 1. No newlines between the method name and `(` — prevents picking up
    //    identifiers from a completely different statement/line.
    // 2. The method name is not a Ruby keyword (return, if, unless, etc.) —
    //    keywords followed by `(` create grouping parens, not method calls.
    let gap = &bytes[end_of_method + 1..paren_pos];
    let has_newline = gap.contains(&b'\n');
    let has_space = gap.iter().any(|&b| b == b' ' || b == b'\t');
    let is_keyword = matches!(
        method_name,
        b"return"
            | b"if"
            | b"unless"
            | b"while"
            | b"until"
            | b"and"
            | b"or"
            | b"not"
            | b"when"
            | b"case"
            | b"elsif"
            | b"yield"
    );
    let is_grouping_paren = method_start > end_of_method || has_newline || is_keyword;
    // When there's a space between the method name and `(`, the `(` starts a
    // grouped subexpression within the argument list: `schedule (Time.now - 60).to_f`.
    // Any chain after `)` (like `.to_f`) is on the grouped expression, NOT on the
    // enclosing call. We must not check chain_contains_tz_safe_method after `)`.
    let is_spaced_paren = has_space && !is_grouping_paren;

    if !is_grouping_paren {
        if SAFE_METHODS.contains(&method_name) {
            return true;
        }

        // The enclosing function itself isn't safe, but check if the CHAIN AFTER
        // the enclosing call's closing `)` contains a safe method.
        // E.g., `Time.to_mongo(Time.local(...)).zone` — `to_mongo` is not safe,
        // but `.zone` after `Time.to_mongo(...)` IS safe.
        // Find the closing `)` that matches the `(` at paren_pos, then scan forward.
        //
        // Skip this check when there's a space between method name and `(`:
        // `schedule (Time.now - 60).to_f` — `.to_f` chains on the grouped
        // expression `(Time.now - 60)`, not on the `schedule` call.
        if !is_spaced_paren {
            let closing_paren = find_matching_close_paren(bytes, paren_pos);
            if let Some(close_pos) = closing_paren {
                if chain_contains_tz_safe_method(bytes, close_pos + 1) {
                    return true;
                }
            }
        }
    }

    // Not safe at this level — try the next enclosing level.
    // E.g., Time.parse(helper_method(Time.now)).utc
    // At level 1: helper_method( is not safe, chain after helper_method(...) is ) — not safe
    // At level 2: Time.parse( is checked, chain after Time.parse(...) is .utc — safe!
    enclosing_call_is_safe_recursive(bytes, paren_pos, max_depth - 1)
}

/// Find the opening `(` that encloses the position `pos` in the source.
/// Scans backward, tracking nested parens/brackets/braces, to find the
/// unmatched `(` that contains this position as an argument.
fn find_enclosing_open_paren(bytes: &[u8], pos: usize) -> Option<usize> {
    let mut i = pos.saturating_sub(1);
    let mut depth = 0u32; // tracks nested closers we need to skip

    while i < bytes.len() {
        match bytes[i] {
            b'(' if depth == 0 => return Some(i),
            b'(' => depth -= 1,
            b')' => depth += 1,
            b'\'' | b'"' => {
                // Skip backward past string literals
                if i == 0 {
                    return None;
                }
                let quote = bytes[i];
                i -= 1;
                while i > 0 && bytes[i] != quote {
                    // Handle escaped quotes: if we see the quote preceded by \, keep going
                    if bytes[i] == quote && i > 0 && bytes[i - 1] == b'\\' {
                        i -= 1;
                    }
                    i -= 1;
                }
                // i is now at the opening quote
            }
            _ => {}
        }
        if i == 0 {
            // Check the byte at position 0
            if bytes[0] == b'(' && depth == 0 {
                return Some(0);
            }
            return None;
        }
        i -= 1;
    }
    None
}

/// Find the position of the closing `)` that matches the opening `(` at `open_pos`.
fn find_matching_close_paren(bytes: &[u8], open_pos: usize) -> Option<usize> {
    let mut pos = open_pos + 1;
    let mut depth = 1u32;
    while pos < bytes.len() && depth > 0 {
        match bytes[pos] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(pos);
                }
            }
            b'\'' | b'"' => {
                let quote = bytes[pos];
                pos += 1;
                while pos < bytes.len() && bytes[pos] != quote {
                    if bytes[pos] == b'\\' {
                        pos += 1;
                    }
                    pos += 1;
                }
            }
            _ => {}
        }
        pos += 1;
    }
    None
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

        // Must see '.' to continue the chain. In RuboCop, `csend` (safe navigation
        // `&.`) is not `send_type?`, so `extract_method_chain` stops at `&.`. We match
        // this by only following regular `.` chains. Since whitespace is skipped above,
        // `&.utc` lands on `&` (not `.`) and correctly returns false here.
        if pos >= bytes.len() || bytes[pos] != b'.' {
            return false;
        }
        pos += 1; // skip the '.'

        // Skip whitespace after dot
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }

        // Read the method name (include ? and ! suffixes)
        let method_start = pos;
        while pos < bytes.len()
            && (bytes[pos].is_ascii_alphanumeric()
                || bytes[pos] == b'_'
                || bytes[pos] == b'?'
                || bytes[pos] == b'!')
        {
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

    #[test]
    fn to_time_flagged_in_strict_mode() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;
        let mut options = HashMap::new();
        options.insert(
            "EnforcedStyle".to_string(),
            serde_yml::Value::String("strict".to_string()),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        // In strict mode, string literal receivers are flagged.
        // Non-string receivers (date_str.to_time) are NOT flagged — RuboCop requires str_type?.
        let fixture = b"\"2005-02-27 23:50\".to_time\n                   ^^^^^^^ Rails/TimeZone: Do not use `String#to_time` without zone. Use `Time.zone.parse` instead.\n\"2005-02-27 23:50\".to_time(:utc)\n                   ^^^^^^^ Rails/TimeZone: Do not use `String#to_time` without zone. Use `Time.zone.parse` instead.\n";
        crate::testutil::assert_cop_offenses_full_with_config(&TimeZone, fixture, config);
    }

    #[test]
    fn to_time_flagged_in_flexible_mode() {
        // RuboCop fires on String#to_time in BOTH strict and flexible mode.
        // Variable receivers are NOT flagged (RuboCop requires str_type? receiver).
        let fixture = b"\"2005-02-27 23:50\".to_time\n                   ^^^^^^^ Rails/TimeZone: Do not use `String#to_time` without zone. Use `Time.zone.parse` instead.\n\"2005-02-27 23:50\".to_time(:utc)\n                   ^^^^^^^ Rails/TimeZone: Do not use `String#to_time` without zone. Use `Time.zone.parse` instead.\n";
        crate::testutil::assert_cop_offenses_full(&TimeZone, fixture);
    }

    #[test]
    fn to_time_not_flagged_for_non_string_receivers() {
        // RuboCop only flags string literal receivers, not variable.to_time
        let source = b"date_str.to_time\nmy_var.to_time\nto_time\n";
        crate::testutil::assert_cop_no_offenses_full(&TimeZone, source);
    }
}
