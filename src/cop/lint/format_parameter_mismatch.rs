use crate::cop::shared::node_type::{
    ARRAY_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE,
    INTERPOLATED_STRING_NODE, KEYWORD_HASH_NODE, SPLAT_NODE, STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for mismatch between format string fields and arguments.
///
/// ## Corpus conformance investigation (2026-03-11)
///
/// **Root causes of FPs (54):**
/// 1. Heredoc format strings — RuboCop explicitly skips heredocs via `heredoc?` check
///    (source starts with `<<`). nitrocop was trying to parse heredoc content.
/// 2. Interpolated string (dstr) with zero format fields — RuboCop skips when
///    `expected_fields == 0 && first_arg.type?(:dstr, :array)`. Common with
///    `format("#{foo}", bar, baz)` where the interpolation IS the format.
/// 3. Zero fields + array RHS in String#% — `"text" % [value]` where string has
///    no format sequences. RuboCop skips when fields=0 and arg is array.
/// 4. format/sprintf with only 1 arg (just the format string, no extra args) —
///    RuboCop requires `arguments.size > 1` to consider it a format call.
///
/// **Fixes applied (round 1):**
/// - Skip heredoc format strings (check opening_loc starts with `<<`)
/// - Require args.len() > 1 for format/sprintf (matches RuboCop)
/// - Skip when zero fields AND format string is interpolated (dstr)
/// - Skip when zero fields AND RHS is array for String#%
///
/// ## Additional conformance investigation (2026-03-11)
///
/// **Root causes of remaining FPs (35) and FNs (13):**
/// 1. Format type character acceptance too broad — nitrocop was treating ANY
///    alphabetic character after `%` as a valid format type. RuboCop only accepts
///    `[bBdiouxXeEfgGaAcps]`. Characters like `%v`, `%n`, `%t`, `%r` are NOT
///    valid Ruby format types. This caused both FPs (over-counting fields) and
///    FNs (wrong field count masking mismatches).
/// 2. String#% splat handling — nitrocop special-cased splats in array RHS,
///    only firing when splat count > expected fields. RuboCop does NOT
///    special-case splats for `%` (only for format/sprintf). It counts child
///    nodes literally and compares directly.
/// 3. Numbered format without valid type — `%1$n` was counted as numbered
///    format even though `n` is not a valid type. RuboCop's SEQUENCE regex
///    requires TYPE at the end for numbered formats.
/// 4. Annotated named format without valid type — `%<name>` without a
///    following type character was counted as named. RuboCop requires TYPE
///    after `%<name>` (only template `%{name}` format has no TYPE requirement).
///
/// **Fixes applied (round 2):**
/// - Restrict format type to `[bBdiouxXeEfgGaAcps]` via `is_format_type()`
/// - Remove splat special-casing for String#% (literal count like RuboCop)
/// - Require valid type for numbered (`%N$X`) and annotated named (`%<name>X`)
///
/// ## Additional conformance investigation (2026-03-14)
///
/// **Root causes of remaining FPs (9) and FNs (19):**
/// 1. Annotated named format with flags/width before `<name>` — e.g.,
///    `%-2<pos>d`, `%06<hex>x`. nitrocop only checked for `<` immediately
///    after `%`, missing cases where flags/width precede the name. RuboCop's
///    SEQUENCE regex supports NAME in multiple positions relative to flags/width.
/// 2. `*N$` dynamic width in numbered formats — `%1$*2$s` has both numbered
///    arg ref `1$` and numbered width ref `*2$`. nitrocop tracked `1$` for
///    max_numbered but ignored `*N$` width refs, causing miscount.
/// 3. `*N$` in initial position — `%*2$s` has `*` followed by `2$`. Without
///    digit_dollar before the `*`, this was parsed as unnumbered `*` + digits,
///    missing the numbered reference. RuboCop treats `*2$` as NUMBER_ARG in
///    WIDTH, and `2$` makes it numbered.
/// 4. Mixed format with `*` and `N$` — `%*.*2$s` mixes unnumbered `*` width
///    with numbered `.*2$` precision, which RuboCop detects as mixed/invalid.
/// 5. Named format `%{name}` with `String#%` and empty array — `"%{foo}" % []`
///    should flag (0 args, expects 1 hash). nitrocop returned early for all
///    named formats with `String#%`.
///
/// **Fixes applied (round 3):**
/// - Check for `<name>` after flags/width/precision in unnumbered path
/// - Track `*N$` width refs in max_numbered for both initial and numbered paths
/// - Detect `*` without `N$` as unnumbered contributor for mixing detection
/// - Flag named `%{name}` format with non-hash RHS in `String#%`
///
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=29, FN=6.
///
/// FP=16 (iruby): Custom DSL method `format 'text/latex' do |obj|` was
/// incorrectly treated as `Kernel#format`. Root cause: `call.block()` includes
/// both `do...end` blocks (BlockNode) and `&blk` arguments (BlockArgumentNode),
/// but RuboCop's Parser gem only counts `&blk` in `arguments.size`, not
/// `do...end` blocks. Fixed by checking `as_block_argument_node()` specifically.
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=13, FN=6.
///
/// Additional investigation after the previous numbering fix showed two
/// RuboCop-1.84.2 behaviors that are easy to misread from the syntax alone:
/// 1. Width/precision interpolations are parsed from the source text, not from
///    the runtime string value. That means `#{1 * 3}` contributes an extra
///    argument because RuboCop's `arity` implementation literally counts `*`
///    characters inside the interpolation source, while `#{padding}` does not.
/// 2. Trailing numbered value refs after width/precision stars (for example
///    `%*1$.*2$3$d`) are not matched by RuboCop's `FormatString::SEQUENCE`
///    regex at all. The no-offense outcome for `String#%` comes from the
///    zero-fields + array-RHS short-circuit, not from true numbered parsing.
///
/// FP=3/FN=6 root causes fixed here:
/// - Preserve interpolation source in width/precision parsing so complex
///   interpolations like `%.#{1 * 3}s` and `%0#{size * 2}X` match RuboCop.
/// - Stop treating line-continued adjacent string literals as a plain format
///   string source; RuboCop does not lint those as a single literal.
/// - Remove the unsupported trailing-`N$` numbered-value parsing so invalid
///   sequences like `%*1$.*0$1$s` fall back to RuboCop's zero-field behavior.
pub struct FormatParameterMismatch;

impl Cop for FormatParameterMismatch {
    fn name(&self) -> &'static str {
        "Lint/FormatParameterMismatch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            HASH_NODE,
            INTERPOLATED_STRING_NODE,
            KEYWORD_HASH_NODE,
            SPLAT_NODE,
            STRING_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for format/sprintf (bare or Kernel.method)
        if (method_name == b"format" || method_name == b"sprintf") && is_format_call(&call) {
            diagnostics.extend(check_format_sprintf(self, source, &call, method_name));
            return;
        }

        // Check for String#% operator
        if method_name == b"%" && call.receiver().is_some() {
            diagnostics.extend(check_string_percent(self, source, &call));
        }
    }
}

/// Returns true if this is a `format(...)` / `sprintf(...)` call (bare or Kernel.format)
fn is_format_call(call: &ruby_prism::CallNode<'_>) -> bool {
    match call.receiver() {
        None => true,
        Some(recv) => {
            recv.as_constant_read_node()
                .is_some_and(|c| c.name().as_slice() == b"Kernel")
                || recv
                    .as_constant_path_node()
                    .is_some_and(|cp| cp.name().is_some_and(|n| n.as_slice() == b"Kernel"))
        }
    }
}

fn check_format_sprintf(
    cop: &FormatParameterMismatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
    method_name: &[u8],
) -> Vec<Diagnostic> {
    let args = match call.arguments() {
        Some(a) => a,
        None => return Vec::new(),
    };

    let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
    // RuboCop requires arguments.size > 1 (format string + at least one arg).
    // In Parser gem, block_pass (&block) is included in arguments but do...end
    // blocks are NOT — they wrap the send node as a separate block node.
    // In Prism, both are in call.block(), but only BlockArgumentNode (&blk)
    // should count as an argument to match RuboCop's behavior.
    let has_block_arg = call
        .block()
        .is_some_and(|b| b.as_block_argument_node().is_some());
    let effective_arg_count = arg_list.len() + usize::from(has_block_arg);
    if effective_arg_count <= 1 {
        return Vec::new();
    }

    let first = &arg_list[0];

    // Skip heredoc format strings (RuboCop behavior)
    if is_heredoc_node(first) {
        return Vec::new();
    }

    // Format string must be a string literal (or interpolated string)
    let fmt_str = extract_format_string(first);
    let fmt_str = match fmt_str {
        Some(s) => s,
        None => return Vec::new(), // Variable or non-literal — can't check
    };

    // Count remaining args (excluding the format string)
    let remaining_args = &arg_list[1..];

    // If any remaining arg is a splat, be conservative for format/sprintf
    let has_splat = remaining_args.iter().any(|a| a.as_splat_node().is_some());

    let arg_count = remaining_args.len() + usize::from(has_block_arg);

    // Parse format sequences
    let parse_result = parse_format_string(&fmt_str.value);
    match parse_result {
        FormatParseResult::Fields(field_count) => {
            // When expected fields is zero and format string is interpolated (dstr)
            // or first arg is array, skip — matches RuboCop's behavior where dynamic
            // content may contain the actual format sequences at runtime
            if field_count.count == 0
                && !field_count.named
                && (fmt_str.contains_interpolation || first.as_array_node().is_some())
            {
                return Vec::new();
            }

            // For named formats (%{name} or %<name>), expect exactly 1 hash arg
            if field_count.named {
                if arg_count != 1 {
                    let method_str = std::str::from_utf8(method_name).unwrap_or("format");
                    let loc = call.message_loc().unwrap_or(call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![cop.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Number of arguments ({}) to `{}` doesn't match the number of fields ({}).",
                            arg_count, method_str, 1
                        ),
                    )];
                }
                return Vec::new();
            }

            if has_splat {
                // With splat, can't know exact count — skip
                return Vec::new();
            }

            if arg_count != field_count.count {
                let method_str = std::str::from_utf8(method_name).unwrap_or("format");
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![cop.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Number of arguments ({}) to `{}` doesn't match the number of fields ({}).",
                        arg_count, method_str, field_count.count
                    ),
                )];
            }
        }
        FormatParseResult::Invalid => {
            let _method_str = std::str::from_utf8(method_name).unwrap_or("format");
            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![cop.diagnostic(
                source,
                line,
                column,
                "Format string is invalid because formatting sequence types (numbered, named or unnumbered) are mixed.".to_string(),
            )];
        }
    }

    Vec::new()
}

fn check_string_percent(
    cop: &FormatParameterMismatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
) -> Vec<Diagnostic> {
    let receiver = call.receiver().unwrap();

    // Skip heredoc receivers (RuboCop behavior)
    if is_heredoc_node(&receiver) {
        return Vec::new();
    }

    // Receiver must be a string literal
    let fmt_str = extract_format_string(&receiver);
    let fmt_str = match fmt_str {
        Some(s) => s,
        None => return Vec::new(),
    };

    let args = match call.arguments() {
        Some(a) => a,
        None => return Vec::new(),
    };
    let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return Vec::new();
    }

    let rhs = &arg_list[0];

    // Parse format sequences
    let parse_result = parse_format_string(&fmt_str.value);
    match parse_result {
        FormatParseResult::Fields(field_count) => {
            if field_count.named {
                // Named formats (%{name}) expect a hash argument.
                // If RHS is an array (not a hash), it's a mismatch.
                if let Some(arr) = rhs.as_array_node() {
                    let arg_count = arr.elements().iter().count();
                    let loc = call.message_loc().unwrap_or(call.location());
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![cop.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Number of arguments ({}) to `String#%` doesn't match the number of fields ({}).",
                            arg_count, 1
                        ),
                    )];
                }
                return Vec::new();
            }

            // When expected fields is zero and first arg is dstr or array,
            // skip — matches RuboCop's offending_node? guard
            if field_count.count == 0
                && (rhs.as_array_node().is_some() || rhs.as_interpolated_string_node().is_some())
            {
                return Vec::new();
            }

            // Also skip when format string is interpolated (dstr) with zero fields
            if field_count.count == 0 && fmt_str.contains_interpolation {
                return Vec::new();
            }

            // RHS must be an array literal for us to check count
            let array_elements = match rhs.as_array_node() {
                Some(arr) => {
                    let elems: Vec<ruby_prism::Node<'_>> = arr.elements().iter().collect();
                    elems
                }
                None => {
                    // Single non-array argument — could be a variable that evaluates to array
                    // For Hash literals, skip (named format)
                    if rhs.as_hash_node().is_some() || rhs.as_keyword_hash_node().is_some() {
                        return Vec::new();
                    }
                    return Vec::new();
                }
            };

            let arg_count = array_elements.len();

            // RuboCop does NOT special-case splats for String#% —
            // it just counts child nodes literally (including splat nodes)
            // and compares against expected fields
            if arg_count != field_count.count {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![cop.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Number of arguments ({}) to `String#%` doesn't match the number of fields ({}).",
                        arg_count, field_count.count
                    ),
                )];
            }
        }
        FormatParseResult::Invalid => {
            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![cop.diagnostic(
                source,
                line,
                column,
                "Format string is invalid because formatting sequence types (numbered, named or unnumbered) are mixed.".to_string(),
            )];
        }
    }

    Vec::new()
}

/// Returns true if the node is a heredoc string (opening starts with `<<`).
fn is_heredoc_node(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(str_node) = node.as_interpolated_string_node() {
        if let Some(open) = str_node.opening_loc() {
            return open.as_slice().starts_with(b"<<");
        }
    }
    if let Some(str_node) = node.as_string_node() {
        if let Some(open) = str_node.opening_loc() {
            return open.as_slice().starts_with(b"<<");
        }
    }
    false
}

struct FormatString {
    value: String,
    contains_interpolation: bool,
}

fn extract_format_string(node: &ruby_prism::Node<'_>) -> Option<FormatString> {
    if has_line_continued_adjacent_literal(node) {
        return None;
    }

    if let Some(s) = node.as_string_node() {
        let val = s.unescaped();
        return Some(FormatString {
            value: String::from_utf8_lossy(val).to_string(),
            contains_interpolation: false,
        });
    }

    if let Some(interp) = node.as_interpolated_string_node() {
        let mut result = String::new();
        let mut has_interp = false;
        for part in interp.parts().iter() {
            if let Some(s) = part.as_string_node() {
                let val = s.unescaped();
                result.push_str(&String::from_utf8_lossy(val));
            } else {
                has_interp = true;
                result.push_str(&String::from_utf8_lossy(part.location().as_slice()));
            }
        }
        return Some(FormatString {
            value: result,
            contains_interpolation: has_interp,
        });
    }

    None
}

fn has_line_continued_adjacent_literal(node: &ruby_prism::Node<'_>) -> bool {
    std::str::from_utf8(node.location().as_slice())
        .unwrap_or("")
        .contains("\\\n")
}

struct FieldCount {
    count: usize,
    named: bool,
}

enum FormatParseResult {
    Fields(FieldCount),
    Invalid,
}

/// Returns true if the byte is a valid Ruby format conversion type character.
/// Matches RuboCop's FormatString::TYPE = [bBdiouxXeEfgGaAcps]
fn is_format_type(b: u8) -> bool {
    matches!(
        b,
        b'b' | b'B'
            | b'd'
            | b'i'
            | b'o'
            | b'u'
            | b'x'
            | b'X'
            | b'e'
            | b'E'
            | b'f'
            | b'g'
            | b'G'
            | b'a'
            | b'A'
            | b'c'
            | b'p'
            | b's'
    )
}

/// Tries to parse `*` possibly followed by `N$` (digit_dollar) for dynamic width/precision.
/// Returns `(advanced_past_star, numbered_ref)` where `numbered_ref` is the `N` if `N$` was found.
fn parse_star_with_optional_dollar(bytes: &[u8], pos: usize) -> (usize, Option<usize>) {
    let len = bytes.len();
    if pos >= len || bytes[pos] != b'*' {
        return (pos, None);
    }
    let mut i = pos + 1; // skip '*'
    // Check for N$ after *
    let digit_start = i;
    while i < len && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > digit_start && i < len && bytes[i] == b'$' {
        // Found *N$ pattern
        let num_str = std::str::from_utf8(&bytes[digit_start..i]).unwrap_or("");
        let n = num_str.parse::<usize>().ok();
        i += 1; // skip '$'
        (i, n)
    } else {
        // Just *, no N$
        (pos + 1, None)
    }
}

/// Parse a `#{...}` interpolation in the source text and return the byte
/// position after the closing `}` plus the number of `*` characters inside the
/// interpolation. RuboCop's `FormatSequence#arity` literally counts `*`
/// characters in the matched source, so `#{1 * 3}` contributes one extra arg
/// while `#{padding}` does not.
fn parse_interpolation(bytes: &[u8], pos: usize) -> Option<(usize, usize)> {
    if pos + 1 >= bytes.len() || bytes[pos] != b'#' || bytes[pos + 1] != b'{' {
        return None;
    }

    let mut i = pos + 2;
    let mut stars = 0;
    while i < bytes.len() {
        if bytes[i] == b'*' {
            stars += 1;
        }
        if bytes[i] == b'}' {
            return Some((i + 1, stars));
        }
        i += 1;
    }

    None
}

fn parse_format_string(fmt: &str) -> FormatParseResult {
    let bytes = fmt.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut count = 0;
    let mut has_numbered = false;
    let mut has_unnumbered = false;
    let mut has_named = false;
    let mut max_numbered = 0;

    while i < len {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        i += 1; // skip '%'

        if i >= len {
            break;
        }

        // `%%` is a literal percent
        if bytes[i] == b'%' {
            i += 1;
            continue;
        }

        // Named template format: %{name} (no type required)
        if bytes[i] == b'{' {
            has_named = true;
            // Skip to closing }
            while i < len && bytes[i] != b'}' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        // Annotated named format immediately after %: %<name>...TYPE
        if bytes[i] == b'<' {
            if let Some(end) = parse_annotated_name(bytes, i) {
                i = end;
                has_named = true;
            }
            continue;
        }

        // Parse flags: [ #0+-] and also digit_dollar (N$) which RuboCop treats as a flag
        let start = i;
        // Skip standard flags
        while i < len && matches!(bytes[i], b'-' | b'+' | b' ' | b'0')
            || (i < len && bytes[i] == b'#' && (i + 1 >= len || bytes[i + 1] != b'{'))
        {
            i += 1;
        }

        // Check for `*` (dynamic width) or width digits
        let mut extra_args = 0;
        // Track whether this sequence has any numbered (*N$) or unnumbered (*) star refs
        let mut seq_has_numbered_star = false;
        let mut seq_has_unnumbered_star = false;
        let mut seq_max_numbered = 0;
        if i < len && bytes[i] == b'*' {
            let (new_i, numbered_ref) = parse_star_with_optional_dollar(bytes, i);
            i = new_i;
            if let Some(n) = numbered_ref {
                // *N$ — numbered width reference
                seq_has_numbered_star = true;
                seq_max_numbered = seq_max_numbered.max(n);
            } else {
                // Plain * — unnumbered extra arg
                seq_has_unnumbered_star = true;
                extra_args += 1;
            }
        } else if let Some((new_i, interp_stars)) = parse_interpolation(bytes, i) {
            i = new_i;
            extra_args += interp_stars;
        } else {
            // Skip width digits
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }

        // Check for `$` (numbered argument, e.g., %1$s where digits before $ are the arg number)
        if i < len && bytes[i] == b'$' {
            // This is a numbered format like %1$s
            // Extract the number from bytes between start and current position
            let num_str = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
            // Remove any flag characters from the front to get the number
            let num_part: String = num_str.chars().filter(|c| c.is_ascii_digit()).collect();
            let parsed_num = num_part.parse::<usize>().ok();
            i += 1; // skip '$'

            // After $, parse the rest of the format specifier
            // Skip flags
            while i < len && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
                i += 1;
            }

            // Skip width (could be * for dynamic width with numbered ref)
            if i < len && bytes[i] == b'*' {
                let (new_i, numbered_ref) = parse_star_with_optional_dollar(bytes, i);
                i = new_i;
                if let Some(n) = numbered_ref {
                    // *N$ width reference in numbered format
                    seq_max_numbered = seq_max_numbered.max(n);
                }
                // Plain * in numbered format — still an unnumbered reference,
                // which means mixing (will be caught by mix_count check)
            } else if let Some((new_i, _interp_stars)) = parse_interpolation(bytes, i) {
                i = new_i;
            } else {
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }

            // Skip precision
            if i < len && bytes[i] == b'.' {
                i += 1;
                if i < len && bytes[i] == b'*' {
                    let (new_i, numbered_ref) = parse_star_with_optional_dollar(bytes, i);
                    i = new_i;
                    if let Some(n) = numbered_ref {
                        seq_max_numbered = seq_max_numbered.max(n);
                    }
                } else if let Some((new_i, _interp_stars)) = parse_interpolation(bytes, i) {
                    i = new_i;
                } else {
                    while i < len && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }

            // Conversion type must be valid
            if i < len && is_format_type(bytes[i]) {
                if let Some(n) = parsed_num {
                    has_numbered = true;
                    max_numbered = max_numbered.max(seq_max_numbered.max(n));
                }
                i += 1;
            }
            continue;
        }

        // Not a numbered format (no $ found). Check for <name> (annotated named
        // format where flags/width precede the name, e.g., %-2<pos>d, %06<hex>x)
        if i < len && bytes[i] == b'<' {
            if let Some(end) = parse_annotated_name(bytes, i) {
                i = end;
                has_named = true;
                continue;
            }
        }

        // Skip precision
        if i < len && bytes[i] == b'.' {
            i += 1;
            if i < len && bytes[i] == b'*' {
                let (new_i, numbered_ref) = parse_star_with_optional_dollar(bytes, i);
                i = new_i;
                if let Some(n) = numbered_ref {
                    seq_has_numbered_star = true;
                    seq_max_numbered = seq_max_numbered.max(n);
                } else {
                    seq_has_unnumbered_star = true;
                    extra_args += 1;
                }
            } else if let Some((new_i, interp_stars)) = parse_interpolation(bytes, i) {
                i = new_i;
                extra_args += interp_stars;
            } else {
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
        }

        // Check for <name> after precision too (e.g., %.3<number>d)
        if i < len && bytes[i] == b'<' {
            if let Some(end) = parse_annotated_name(bytes, i) {
                i = end;
                has_named = true;
                continue;
            }
        }

        // Conversion specifier — must be a valid Ruby format type.
        if i < len && is_format_type(bytes[i]) {
            // A sequence can contribute to both numbered and unnumbered if it
            // mixes *N$ with plain * (e.g., %*.*2$s). Track each contribution.
            if seq_has_unnumbered_star || extra_args > 0 {
                has_unnumbered = true;
                count += 1 + extra_args;
            } else if seq_has_numbered_star {
                // Entirely numbered (all stars are *N$), don't count as unnumbered
                has_numbered = true;
                max_numbered = max_numbered.max(seq_max_numbered);
            } else {
                // No stars at all — regular unnumbered format
                has_unnumbered = true;
                count += 1;
            }
            i += 1;
        }
    }

    // Check for mixing
    let mix_count = [has_named, has_numbered, has_unnumbered]
        .iter()
        .filter(|&&b| b)
        .count();
    if mix_count > 1 {
        return FormatParseResult::Invalid;
    }

    if has_named {
        return FormatParseResult::Fields(FieldCount {
            count: 1,
            named: true,
        });
    }

    if has_numbered {
        return FormatParseResult::Fields(FieldCount {
            count: max_numbered,
            named: false,
        });
    }

    FormatParseResult::Fields(FieldCount {
        count,
        named: false,
    })
}

/// Parse an annotated named format: `<name>` followed by optional flags/width/precision and
/// a required TYPE character. Returns the position after the TYPE if valid, None otherwise.
fn parse_annotated_name(bytes: &[u8], start: usize) -> Option<usize> {
    let len = bytes.len();
    if start >= len || bytes[start] != b'<' {
        return None;
    }
    // Skip to closing >
    let mut i = start + 1;
    while i < len && bytes[i] != b'>' {
        i += 1;
    }
    if i >= len {
        return None;
    }
    i += 1; // skip '>'

    // After >, may have more_flags, width, precision before TYPE
    // Skip flags
    while i < len && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
        i += 1;
    }
    // Skip width
    while i < len && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Skip precision
    if i < len && bytes[i] == b'.' {
        i += 1;
        while i < len && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    // TYPE is required for annotated named format
    if i < len && is_format_type(bytes[i]) {
        Some(i + 1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        FormatParameterMismatch,
        "cops/lint/format_parameter_mismatch"
    );
}
