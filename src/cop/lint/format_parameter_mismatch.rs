use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FormatParameterMismatch;

impl Cop for FormatParameterMismatch {
    fn name(&self) -> &'static str {
        "Lint/FormatParameterMismatch"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check for format/sprintf (bare or Kernel.method)
        if (method_name == b"format" || method_name == b"sprintf")
            && is_format_call(&call)
        {
            return check_format_sprintf(self, source, &call, method_name);
        }

        // Check for String#% operator
        if method_name == b"%" && call.receiver().is_some() {
            return check_string_percent(self, source, &call);
        }

        Vec::new()
    }
}

/// Returns true if this is a `format(...)` / `sprintf(...)` call (bare or Kernel.format)
fn is_format_call(call: &ruby_prism::CallNode<'_>) -> bool {
    match call.receiver() {
        None => true,
        Some(recv) => {
            recv.as_constant_read_node()
                .is_some_and(|c| c.name().as_slice() == b"Kernel")
                || recv.as_constant_path_node().is_some_and(|cp| {
                    cp.name()
                        .is_some_and(|n| n.as_slice() == b"Kernel")
                })
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
    if arg_list.is_empty() {
        return Vec::new();
    }

    let first = &arg_list[0];

    // Format string must be a string literal (or interpolated string)
    let fmt_str = extract_format_string(first);
    let fmt_str = match fmt_str {
        Some(s) => s,
        None => return Vec::new(), // Variable or non-literal — can't check
    };

    // If the format string contains interpolation that could affect format sequences, bail
    if fmt_str.contains_interpolation {
        // Still try to count sequences that don't depend on interpolation
        // but if we can't determine the count reliably, bail
        if fmt_str.has_format_affecting_interpolation {
            return Vec::new();
        }
    }

    // Count remaining args (excluding the format string)
    let remaining_args = &arg_list[1..];

    // If any remaining arg is a splat, be conservative for format/sprintf
    let has_splat = remaining_args.iter().any(|a| a.as_splat_node().is_some());

    let arg_count = remaining_args.len();

    // Parse format sequences
    let parse_result = parse_format_string(&fmt_str.value);
    match parse_result {
        FormatParseResult::Fields(field_count) => {
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
            let method_str = std::str::from_utf8(method_name).unwrap_or("format");
            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![cop.diagnostic(
                source,
                line,
                column,
                "Format string is invalid because formatting sequence types (numbered, named or unnumbered) are mixed.".to_string(),
            )];
        }
        FormatParseResult::Unknown => {}
    }

    Vec::new()
}

fn check_string_percent(
    cop: &FormatParameterMismatch,
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
) -> Vec<Diagnostic> {
    let receiver = call.receiver().unwrap();

    // Receiver must be a string literal
    let fmt_str = extract_format_string(&receiver);
    let fmt_str = match fmt_str {
        Some(s) => s,
        None => return Vec::new(),
    };

    if fmt_str.contains_interpolation && fmt_str.has_format_affecting_interpolation {
        return Vec::new();
    }

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
                // Named formats expect a hash — don't check further
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

            let has_splat = array_elements.iter().any(|e| e.as_splat_node().is_some());

            let arg_count = array_elements.len();

            if has_splat && arg_count > field_count.count {
                // Splat with more args than fields — always an error for String#%
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

            if !has_splat && arg_count != field_count.count {
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
        FormatParseResult::Unknown => {}
    }

    Vec::new()
}

struct FormatString {
    value: String,
    contains_interpolation: bool,
    has_format_affecting_interpolation: bool,
}

fn extract_format_string(node: &ruby_prism::Node<'_>) -> Option<FormatString> {
    if let Some(s) = node.as_string_node() {
        let val = s.unescaped();
        return Some(FormatString {
            value: String::from_utf8_lossy(&val).to_string(),
            contains_interpolation: false,
            has_format_affecting_interpolation: false,
        });
    }

    if let Some(interp) = node.as_interpolated_string_node() {
        let mut result = String::new();
        let mut has_interp = false;
        let mut format_affecting = false;
        for part in interp.parts().iter() {
            if let Some(s) = part.as_string_node() {
                let val = s.unescaped();
                result.push_str(&String::from_utf8_lossy(&val));
            } else {
                has_interp = true;
                // Check if the interpolation could affect format parsing
                // If the string part right before this ends with `%` or `%-` etc.,
                // the interpolation could be part of a format specifier
                if result.ends_with('%')
                    || result.ends_with("%-")
                    || result.ends_with("%+")
                    || result.ends_with("%0")
                    || result.ends_with("%.")
                {
                    format_affecting = true;
                }
            }
        }
        return Some(FormatString {
            value: result,
            contains_interpolation: has_interp,
            has_format_affecting_interpolation: format_affecting,
        });
    }

    None
}

struct FieldCount {
    count: usize,
    named: bool,
}

enum FormatParseResult {
    Fields(FieldCount),
    Invalid,
    Unknown,
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

        // Named format: %{name} or %<name>
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

        if bytes[i] == b'<' {
            has_named = true;
            // Skip to closing >
            while i < len && bytes[i] != b'>' {
                i += 1;
            }
            if i < len {
                i += 1;
                // Skip the conversion specifier after >
                if i < len && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
            }
            continue;
        }

        // Check for numbered: %1$s, %2$d, etc.
        // Flags, width, precision, then conversion
        let start = i;
        // Skip flags
        while i < len && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
            i += 1;
        }

        // Check for `*` (dynamic width — counts as an extra arg)
        let mut extra_args = 0;
        if i < len && bytes[i] == b'*' {
            extra_args += 1;
            i += 1;
        } else {
            // Skip width digits
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }

        // Check for `$` (numbered argument)
        if i < len && bytes[i] == b'$' {
            // This is a numbered format like %1$s
            // Extract the number
            let num_str = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
            // Remove any flag characters from the front to get the number
            let num_part: String = num_str.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num_part.parse::<usize>() {
                has_numbered = true;
                if n > max_numbered {
                    max_numbered = n;
                }
            }
            i += 1; // skip '$'
            // Skip the rest of the format specifier after $
            // Skip flags again
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
            // Skip conversion
            if i < len && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            continue;
        }

        // Skip precision
        if i < len && bytes[i] == b'.' {
            i += 1;
            if i < len && bytes[i] == b'*' {
                extra_args += 1;
                i += 1;
            } else {
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
        }

        // Skip conversion specifier
        if i < len && bytes[i].is_ascii_alphabetic() {
            has_unnumbered = true;
            count += 1 + extra_args;
            i += 1;
        } else if i >= len || !bytes[i].is_ascii_alphabetic() {
            // Could be something like `%` at end — skip
            if extra_args > 0 {
                has_unnumbered = true;
                count += extra_args;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FormatParameterMismatch, "cops/lint/format_parameter_mismatch");
}
