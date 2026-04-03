use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE, SPLAT_NODE,
    STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus follow-up (2026-03-30): fixed the remaining FN clusters while
/// preserving the existing receiver, block, and splat safeguards.
///
/// 1. RuboCop only limits the single-argument shortcut (`format('x')`,
///    `format(CONSTANT)`) to bare/`Kernel` receivers. The multi-argument literal
///    formatter still flags receiver calls like `@parameter.format("%s", "x")`.
///    nitrocop was applying the receiver restriction too broadly.
/// 2. Prism stores both real blocks (`do ... end`, `{}`) and block passes
///    (`&:inspect`, `&block`) in `call.block()`. The earlier blanket skip
///    suppressed real offenses like `format 'text/html' do |obj| ... end`.
///    Fixed by skipping only `BlockArgumentNode` forms.
/// 3. RuboCop also flags named keyword formats like `%-38<form_uuid>s` and
///    `%{summary}` when every referenced hash value is literal. nitrocop's
///    parser rejected named placeholders entirely, and `%s` replacements could
///    not rebuild interpolated string/symbol arguments. Fixed by resolving a
///    single literal hash argument by key and by reconstructing interpolated
///    string content before quoting the final replacement.
pub struct RedundantFormat;

/// Check if a node is a literal that can be used with %s format specifier.
fn is_acceptable_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
}

/// Check if a node is an integer-compatible literal (for %d/%i/%u).
fn is_integer_compatible(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_integer_node().is_some() || node.as_float_node().is_some() {
        return true;
    }
    if let Some(s) = node.as_string_node() {
        let content = s.content_loc().as_slice();
        if let Ok(text) = std::str::from_utf8(content) {
            return text.parse::<i64>().is_ok();
        }
    }
    false
}

/// Check if a node is a float-compatible literal (for %f).
fn is_float_compatible(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_integer_node().is_some() || node.as_float_node().is_some() {
        return true;
    }
    if let Some(s) = node.as_string_node() {
        let content = s.content_loc().as_slice();
        if let Ok(text) = std::str::from_utf8(content) {
            return text.parse::<f64>().is_ok();
        }
    }
    false
}

/// Get a literal's string representation for %s.
/// Uses source text for integer nodes since ruby_prism::Integer is not a primitive.
fn literal_to_string(node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(s) = node.as_string_node() {
        let content = s.content_loc().as_slice();
        return std::str::from_utf8(content).ok().map(|v| v.to_string());
    }
    if let Some(s) = node.as_interpolated_string_node() {
        let mut content = String::new();
        for part in s.parts().iter() {
            let text = std::str::from_utf8(part.location().as_slice()).ok()?;
            content.push_str(text);
        }
        return Some(content);
    }
    if let Some(sym) = node.as_symbol_node() {
        let val = sym.unescaped();
        return std::str::from_utf8(val).ok().map(|v| v.to_string());
    }
    if let Some(sym) = node.as_interpolated_symbol_node() {
        let mut content = String::new();
        for part in sym.parts().iter() {
            let text = std::str::from_utf8(part.location().as_slice()).ok()?;
            content.push_str(text);
        }
        return Some(content);
    }
    if node.as_integer_node().is_some() {
        let src = node.location().as_slice();
        return std::str::from_utf8(src).ok().map(|v| v.to_string());
    }
    if let Some(f) = node.as_float_node() {
        return Some(format!("{}", f.value()));
    }
    if node.as_true_node().is_some() {
        return Some("true".to_string());
    }
    if node.as_false_node().is_some() {
        return Some("false".to_string());
    }
    if node.as_nil_node().is_some() {
        return Some("".to_string());
    }
    None
}

/// Get integer value from a literal node.
/// Parses from source text since ruby_prism::Integer is not a primitive i64.
fn literal_to_integer(node: &ruby_prism::Node<'_>) -> Option<i64> {
    if node.as_integer_node().is_some() {
        let src = node.location().as_slice();
        let s = std::str::from_utf8(src).ok()?;
        return s.parse::<i64>().ok();
    }
    if let Some(f) = node.as_float_node() {
        return Some(f.value() as i64);
    }
    if let Some(s) = node.as_string_node() {
        let content = s.content_loc().as_slice();
        if let Ok(text) = std::str::from_utf8(content) {
            return text.parse::<i64>().ok();
        }
    }
    None
}

/// Get float value from a literal node.
fn literal_to_float(node: &ruby_prism::Node<'_>) -> Option<f64> {
    if node.as_integer_node().is_some() {
        let src = node.location().as_slice();
        let s = std::str::from_utf8(src).ok()?;
        return s.parse::<f64>().ok();
    }
    if let Some(f) = node.as_float_node() {
        return Some(f.value());
    }
    if let Some(s) = node.as_string_node() {
        let content = s.content_loc().as_slice();
        let text = std::str::from_utf8(content).ok()?;
        return text.parse::<f64>().ok();
    }
    None
}

/// Represents a parsed format specifier.
struct FormatSpec {
    spec_type: u8,
    width: Option<i32>,
    precision: Option<usize>,
    flags: Vec<u8>,
    name: Option<String>,
    start: usize,
    end: usize,
}

type AnnotatedNameParse = (String, Vec<u8>, Option<u32>, Option<usize>, u8, usize);

/// Parse simple format specifiers from a format string.
/// Returns None if the string contains specifiers we can't handle.
fn parse_simple_format_specs(fmt: &str) -> Option<Vec<FormatSpec>> {
    let bytes = fmt.as_bytes();
    let mut specs = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        if i >= bytes.len() {
            return None;
        }
        if bytes[i] == b'%' {
            i += 1;
            continue;
        }
        let pos_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i > pos_start && i < bytes.len() && bytes[i] == b'$' {
            return None;
        }
        i = pos_start;

        let mut flags = Vec::new();
        while i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
            flags.push(bytes[i]);
            i += 1;
        }

        let width_before = parse_static_width(bytes, &mut i)?;
        let precision_before = parse_static_precision(bytes, &mut i)?;

        if i < bytes.len() && bytes[i] == b'{' {
            let (name, end) = parse_template_name(bytes, i)?;
            specs.push(FormatSpec {
                spec_type: b's',
                width: width_before.map(|width| signed_width(width, &flags)),
                precision: precision_before,
                flags,
                name: Some(name),
                start,
                end,
            });
            i = end;
            continue;
        }

        if i < bytes.len() && bytes[i] == b'<' {
            let (name, more_flags, width_after, precision_after, spec_type, end) =
                parse_annotated_name(bytes, i)?;
            if width_before.is_some() && width_after.is_some() {
                return None;
            }
            if precision_before.is_some() && precision_after.is_some() {
                return None;
            }
            flags.extend(more_flags);
            specs.push(FormatSpec {
                spec_type,
                width: width_before
                    .or(width_after)
                    .map(|width| signed_width(width, &flags)),
                precision: precision_before.or(precision_after),
                flags,
                name: Some(name),
                start,
                end,
            });
            i = end;
            continue;
        }

        let width = width_before.map(|width| signed_width(width, &flags));
        let precision = precision_before;
        if i >= bytes.len() {
            return None;
        }
        let spec_type = bytes[i];
        if !matches!(
            spec_type,
            b's' | b'd'
                | b'i'
                | b'u'
                | b'f'
                | b'g'
                | b'e'
                | b'x'
                | b'X'
                | b'o'
                | b'b'
                | b'B'
                | b'c'
                | b'p'
                | b'a'
                | b'A'
                | b'E'
                | b'G'
        ) {
            return None;
        }
        i += 1;

        specs.push(FormatSpec {
            spec_type,
            width,
            precision,
            flags,
            name: None,
            start,
            end: i,
        });
    }

    Some(specs)
}

fn parse_static_width(bytes: &[u8], i: &mut usize) -> Option<Option<u32>> {
    if *i < bytes.len() && bytes[*i] == b'*' {
        return None;
    }

    let start = *i;
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }

    if *i == start {
        return Some(None);
    }

    let width = std::str::from_utf8(&bytes[start..*i]).ok()?.parse().ok()?;
    Some(Some(width))
}

fn parse_static_precision(bytes: &[u8], i: &mut usize) -> Option<Option<usize>> {
    if *i >= bytes.len() || bytes[*i] != b'.' {
        return Some(None);
    }

    *i += 1;
    if *i < bytes.len() && bytes[*i] == b'*' {
        return None;
    }

    let start = *i;
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }

    if *i == start {
        return Some(Some(0));
    }

    let precision = std::str::from_utf8(&bytes[start..*i]).ok()?.parse().ok()?;
    Some(Some(precision))
}

fn parse_template_name(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    if start >= bytes.len() || bytes[start] != b'{' {
        return None;
    }

    let mut i = start + 1;
    let name_start = i;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    if i == name_start || i >= bytes.len() || bytes[i] != b'}' {
        return None;
    }

    let name = std::str::from_utf8(&bytes[name_start..i]).ok()?.to_string();
    Some((name, i + 1))
}

fn parse_annotated_name(bytes: &[u8], start: usize) -> Option<AnnotatedNameParse> {
    if start >= bytes.len() || bytes[start] != b'<' {
        return None;
    }

    let mut i = start + 1;
    let name_start = i;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    if i == name_start || i >= bytes.len() || bytes[i] != b'>' {
        return None;
    }

    let name = std::str::from_utf8(&bytes[name_start..i]).ok()?.to_string();
    i += 1;

    let mut flags = Vec::new();
    while i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b' ' | b'0' | b'#') {
        flags.push(bytes[i]);
        i += 1;
    }

    let width = parse_static_width(bytes, &mut i)?;
    let precision = parse_static_precision(bytes, &mut i)?;
    if i >= bytes.len() {
        return None;
    }

    let spec_type = bytes[i];
    if !matches!(
        spec_type,
        b's' | b'd'
            | b'i'
            | b'u'
            | b'f'
            | b'g'
            | b'e'
            | b'x'
            | b'X'
            | b'o'
            | b'b'
            | b'B'
            | b'c'
            | b'p'
            | b'a'
            | b'A'
            | b'E'
            | b'G'
    ) {
        return None;
    }

    Some((name, flags, width, precision, spec_type, i + 1))
}

fn signed_width(width: u32, flags: &[u8]) -> i32 {
    let width = width as i32;
    if flags.contains(&b'-') { -width } else { width }
}

/// Check if an argument matches its format specifier type.
fn arg_matches_spec(spec: &FormatSpec, node: &ruby_prism::Node<'_>) -> bool {
    match spec.spec_type {
        b's' => {
            if (spec.width.is_some() || spec.precision.is_some())
                && (node.as_interpolated_string_node().is_some()
                    || node.as_interpolated_symbol_node().is_some())
            {
                return false;
            }
            is_acceptable_literal(node)
        }
        b'd' | b'i' | b'u' => is_integer_compatible(node),
        b'f' => is_float_compatible(node),
        _ => false,
    }
}

/// Format a single value according to a format spec.
fn format_value(spec: &FormatSpec, node: &ruby_prism::Node<'_>) -> Option<String> {
    match spec.spec_type {
        b's' => {
            let val = literal_to_string(node)?;
            apply_width_padding(&val, spec)
        }
        b'd' | b'i' | b'u' => {
            let int_val = literal_to_integer(node)?;
            let mut formatted = if spec.flags.contains(&b'+') && int_val >= 0 {
                format!("+{}", int_val)
            } else if spec.flags.contains(&b' ') && int_val >= 0 {
                format!(" {}", int_val)
            } else {
                format!("{}", int_val)
            };

            if let Some(prec) = spec.precision {
                if prec == 0 && int_val == 0 {
                    formatted = String::new();
                } else {
                    let is_neg = int_val < 0;
                    let prefix_len = usize::from(
                        is_neg || formatted.starts_with('+') || formatted.starts_with(' '),
                    );
                    let digits = &formatted[prefix_len..];
                    if digits.len() < prec {
                        let padded = format!("{:0>width$}", digits, width = prec);
                        if prefix_len > 0 {
                            formatted = format!("{}{}", &formatted[..prefix_len], padded);
                        } else {
                            formatted = padded;
                        }
                    }
                }
            }

            if spec.flags.contains(&b'0') && !spec.flags.contains(&b'-') {
                if let Some(w) = spec.width {
                    let w = w.unsigned_abs() as usize;
                    if formatted.len() < w {
                        let is_neg = formatted.starts_with('-');
                        let digits = if is_neg {
                            &formatted[1..]
                        } else {
                            &formatted[..]
                        };
                        let padded =
                            format!("{:0>width$}", digits, width = w - usize::from(is_neg));
                        formatted = if is_neg {
                            format!("-{}", padded)
                        } else {
                            padded
                        };
                    }
                }
            }

            apply_width_padding(&formatted, spec)
        }
        b'f' => {
            let float_val = literal_to_float(node)?;
            let prec = spec.precision.unwrap_or(6);
            let formatted = format!("{:.prec$}", float_val, prec = prec);
            apply_width_padding(&formatted, spec)
        }
        _ => None,
    }
}

/// Apply width padding to a formatted string.
fn apply_width_padding(s: &str, spec: &FormatSpec) -> Option<String> {
    if let Some(w) = spec.width {
        let abs_w = w.unsigned_abs() as usize;
        if s.len() < abs_w {
            if w < 0 || spec.flags.contains(&b'-') {
                Some(format!("{:<width$}", s, width = abs_w))
            } else {
                Some(format!("{:>width$}", s, width = abs_w))
            }
        } else {
            Some(s.to_string())
        }
    } else {
        Some(s.to_string())
    }
}

/// Check if any argument in the arg list is a splat or double-splat.
fn has_splat_arg(args: &[ruby_prism::Node<'_>]) -> bool {
    for arg in args {
        if arg.as_splat_node().is_some() {
            return true;
        }
        if let Some(kh) = arg.as_keyword_hash_node() {
            for elem in kh.elements().iter() {
                if elem.as_assoc_splat_node().is_some() {
                    return true;
                }
            }
        }
    }
    false
}

fn is_hash_arg(node: &ruby_prism::Node<'_>) -> bool {
    node.as_keyword_hash_node().is_some() || node.as_hash_node().is_some()
}

fn hash_value_for_name<'a>(
    hash_arg: &ruby_prism::Node<'a>,
    name: &str,
) -> Option<ruby_prism::Node<'a>> {
    let key = name.as_bytes();

    if let Some(keyword_hash) = hash_arg.as_keyword_hash_node() {
        for element in keyword_hash.elements().iter() {
            if let Some(assoc) = element.as_assoc_node() {
                if let Some(symbol) = assoc.key().as_symbol_node() {
                    if symbol.unescaped() == key {
                        return Some(assoc.value());
                    }
                }
            }
        }
    }

    if let Some(hash) = hash_arg.as_hash_node() {
        for element in hash.elements().iter() {
            if let Some(assoc) = element.as_assoc_node() {
                if let Some(symbol) = assoc.key().as_symbol_node() {
                    if symbol.unescaped() == key {
                        return Some(assoc.value());
                    }
                }
            }
        }
    }

    None
}

enum ResolvedArgument<'a> {
    Positional(usize),
    Named(ruby_prism::Node<'a>),
}

impl<'a> ResolvedArgument<'a> {
    fn node<'b>(&'b self, format_args: &'b [ruby_prism::Node<'a>]) -> &'b ruby_prism::Node<'a> {
        match self {
            Self::Positional(index) => &format_args[*index],
            Self::Named(node) => node,
        }
    }
}

fn receiverless_or_kernel(call: &ruby_prism::CallNode<'_>) -> bool {
    match call.receiver() {
        None => true,
        Some(receiver) => {
            if let Some(cr) = receiver.as_constant_read_node() {
                return cr.name().as_slice() == b"Kernel";
            }

            if let Some(cp) = receiver.as_constant_path_node() {
                return cp.parent().is_none()
                    && cp
                        .name()
                        .map(|name| name.as_slice() == b"Kernel")
                        .unwrap_or(false);
            }

            false
        }
    }
}

fn has_block_argument(call: &ruby_prism::CallNode<'_>) -> bool {
    call.block()
        .is_some_and(|block| block.as_block_argument_node().is_some())
}

/// Escape control characters in a string for display.
fn escape_control_chars(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\x07' => result.push_str("\\a"),
            '\x08' => result.push_str("\\b"),
            '\t' => result.push_str("\\t"),
            '\n' => result.push_str("\\n"),
            '\x0B' => result.push_str("\\v"),
            '\x0C' => result.push_str("\\f"),
            '\r' => result.push_str("\\r"),
            '\x1B' => result.push_str("\\e"),
            c if c.is_control() => {
                result.push_str(&format!("\\x{:02X}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

impl Cop for RedundantFormat {
    fn name(&self) -> &'static str {
        "Style/RedundantFormat"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTERPOLATED_STRING_NODE,
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

        let method_bytes = call.name().as_slice();
        if method_bytes != b"format" && method_bytes != b"sprintf" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check for splat/double-splat arguments
        if has_splat_arg(&arg_list) {
            return;
        }

        let method_str = std::str::from_utf8(method_bytes).unwrap_or("format");

        if arg_list.len() == 1 {
            if has_block_argument(&call) {
                return;
            }

            if !receiverless_or_kernel(&call) {
                return;
            }

            let arg = &arg_list[0];

            // Single string/dstr argument
            if arg.as_string_node().is_some() || arg.as_interpolated_string_node().is_some() {
                let arg_src = std::str::from_utf8(arg.location().as_slice()).unwrap_or("");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{arg_src}` directly instead of `{method_str}`."),
                ));
                return;
            }

            // Single constant argument: format(FORMAT), format(Foo::BAR)
            if arg.as_constant_read_node().is_some() || arg.as_constant_path_node().is_some() {
                let arg_src = std::str::from_utf8(arg.location().as_slice()).unwrap_or("");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{arg_src}` directly instead of `{method_str}`."),
                ));
                return;
            }
        }

        // Multi-arg: format('%s %s', 'foo', 'bar') — detect unnecessary fields
        self.detect_unnecessary_fields(source, &call, &arg_list, method_str, diagnostics);
    }
}

impl RedundantFormat {
    fn resolve_format_arguments<'a>(
        &self,
        specs: &[FormatSpec],
        format_args: &[ruby_prism::Node<'a>],
    ) -> Option<Vec<ResolvedArgument<'a>>> {
        if specs.iter().all(|spec| spec.name.is_none()) {
            if specs.len() != format_args.len() {
                return None;
            }
            if specs
                .iter()
                .zip(format_args.iter())
                .all(|(spec, arg)| arg_matches_spec(spec, arg))
            {
                return Some(
                    (0..format_args.len())
                        .map(ResolvedArgument::Positional)
                        .collect(),
                );
            }
            return None;
        }

        let hash_indices: Vec<_> = format_args
            .iter()
            .enumerate()
            .filter(|(_, arg)| is_hash_arg(arg))
            .map(|(index, _)| index)
            .collect();
        if hash_indices.len() != 1 {
            return None;
        }

        let positional_indices: Vec<_> = format_args
            .iter()
            .enumerate()
            .filter(|(_, arg)| !is_hash_arg(arg))
            .map(|(index, _)| index)
            .collect();
        if specs.iter().filter(|spec| spec.name.is_none()).count() != positional_indices.len() {
            return None;
        }

        let hash_index = hash_indices[0];
        let mut positional_index = 0;
        let mut resolved = Vec::with_capacity(specs.len());

        for spec in specs {
            let resolved_arg = if let Some(name) = &spec.name {
                ResolvedArgument::Named(hash_value_for_name(&format_args[hash_index], name)?)
            } else {
                let arg = *positional_indices.get(positional_index)?;
                positional_index += 1;
                ResolvedArgument::Positional(arg)
            };

            if !arg_matches_spec(spec, resolved_arg.node(format_args)) {
                return None;
            }
            resolved.push(resolved_arg);
        }

        Some(resolved)
    }

    fn detect_unnecessary_fields(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        arg_list: &[ruby_prism::Node<'_>],
        method_str: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if arg_list.len() < 2 {
            return;
        }

        // First arg must be a plain string (not interpolated)
        let fmt_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return,
        };

        let fmt_content = fmt_node.content_loc().as_slice();
        let fmt_str = match std::str::from_utf8(fmt_content) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Parse format specifiers — reject complex patterns
        let specs = match parse_simple_format_specs(fmt_str) {
            Some(s) if !s.is_empty() => s,
            _ => return,
        };

        let format_args = &arg_list[1..];
        let resolved_args = match self.resolve_format_arguments(&specs, format_args) {
            Some(args) => args,
            None => return,
        };

        // Compute the formatted result
        let mut parts = Vec::new();
        let mut last_end = 0;
        for (spec, arg) in specs.iter().zip(resolved_args.iter()) {
            if spec.start > last_end {
                parts.push(fmt_str[last_end..spec.start].to_string());
            }
            match format_value(spec, arg.node(format_args)) {
                Some(s) => parts.push(s),
                None => return,
            }
            last_end = spec.end;
        }

        if last_end < fmt_str.len() {
            parts.push(fmt_str[last_end..].to_string());
        }

        let result = parts.join("");
        let has_interpolation = resolved_args.iter().any(|arg| {
            let node = arg.node(format_args);
            node.as_interpolated_string_node().is_some()
                || node.as_interpolated_symbol_node().is_some()
        });
        let escaped = escape_control_chars(&result);
        let format_uses_double_quotes = fmt_node
            .location()
            .as_slice()
            .first()
            .is_some_and(|byte| *byte == b'"');

        let quoted = if has_interpolation
            || escaped.contains('\\')
            || escaped != result
            || format_uses_double_quotes
        {
            format!("\"{}\"", escaped)
        } else {
            format!("'{}'", escaped)
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}` directly instead of `{}`.", quoted, method_str),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantFormat, "cops/style/redundant_format");
}
