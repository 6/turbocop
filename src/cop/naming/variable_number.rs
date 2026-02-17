use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct VariableNumber;

const DEFAULT_ALLOWED: &[&str] = &[
    "TLS1_1", "TLS1_2", "capture3", "iso8601", "rfc1123_date",
    "rfc822", "rfc2822", "rfc3339", "x86_64",
];

impl Cop for VariableNumber {
    fn name(&self) -> &'static str {
        "Naming/VariableNumber"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "normalcase");
        let check_method_names = config.get_bool("CheckMethodNames", true);
        let check_symbols = config.get_bool("CheckSymbols", true);
        let allowed = config.get_string_array("AllowedIdentifiers");
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        let allowed_ids: Vec<String> = allowed.unwrap_or_else(|| {
            DEFAULT_ALLOWED.iter().map(|s| s.to_string()).collect()
        });

        let allowed_pats: Vec<String> = allowed_patterns.unwrap_or_default();

        // Check local variable writes
        if let Some(lvar) = node.as_local_variable_write_node() {
            let name = lvar.name().as_slice();
            let name_str = std::str::from_utf8(name).unwrap_or("");
            if !is_allowed(name_str, &allowed_ids, &allowed_pats) {
                if let Some(diag) = check_number_style(self, source, name_str, &lvar.name_loc(), enforced_style, "variable") {
                    return vec![diag];
                }
            }
        }

        // Check instance variable writes
        if let Some(ivar) = node.as_instance_variable_write_node() {
            let name = ivar.name().as_slice();
            let name_str = std::str::from_utf8(name).unwrap_or("");
            // Strip leading @
            let bare = name_str.trim_start_matches('@');
            if !is_allowed(bare, &allowed_ids, &allowed_pats) {
                if let Some(diag) = check_number_style(self, source, bare, &ivar.name_loc(), enforced_style, "variable") {
                    return vec![diag];
                }
            }
        }

        // Check class variable writes
        if let Some(cvar) = node.as_class_variable_write_node() {
            let name = cvar.name().as_slice();
            let name_str = std::str::from_utf8(name).unwrap_or("");
            let bare = name_str.trim_start_matches('@');
            if !is_allowed(bare, &allowed_ids, &allowed_pats) {
                if let Some(diag) = check_number_style(self, source, bare, &cvar.name_loc(), enforced_style, "variable") {
                    return vec![diag];
                }
            }
        }

        // Check global variable writes
        if let Some(gvar) = node.as_global_variable_write_node() {
            let name = gvar.name().as_slice();
            let name_str = std::str::from_utf8(name).unwrap_or("");
            let bare = name_str.trim_start_matches('$');
            if !is_allowed(bare, &allowed_ids, &allowed_pats) {
                if let Some(diag) = check_number_style(self, source, bare, &gvar.name_loc(), enforced_style, "variable") {
                    return vec![diag];
                }
            }
        }

        // Check method names (def)
        if check_method_names {
            if let Some(def_node) = node.as_def_node() {
                let name = def_node.name().as_slice();
                let name_str = std::str::from_utf8(name).unwrap_or("");
                if !is_allowed(name_str, &allowed_ids, &allowed_pats) {
                    if let Some(diag) = check_number_style(self, source, name_str, &def_node.name_loc(), enforced_style, "method name") {
                        return vec![diag];
                    }
                }
            }
        }

        // Check symbols
        if check_symbols {
            if let Some(sym) = node.as_symbol_node() {
                let name = sym.unescaped();
                let name_str = std::str::from_utf8(&name).unwrap_or("");
                if !is_allowed(name_str, &allowed_ids, &allowed_pats) {
                    if let Some(diag) = check_number_style(self, source, name_str, &sym.value_loc().unwrap_or(sym.location()), enforced_style, "symbol") {
                        return vec![diag];
                    }
                }
            }
        }

        // Check method parameters
        if let Some(param) = node.as_required_parameter_node() {
            let name = param.name().as_slice();
            let name_str = std::str::from_utf8(name).unwrap_or("");
            if !is_allowed(name_str, &allowed_ids, &allowed_pats) {
                if let Some(diag) = check_number_style(self, source, name_str, &param.location(), enforced_style, "variable") {
                    return vec![diag];
                }
            }
        }
        if let Some(param) = node.as_optional_parameter_node() {
            let name = param.name().as_slice();
            let name_str = std::str::from_utf8(name).unwrap_or("");
            if !is_allowed(name_str, &allowed_ids, &allowed_pats) {
                if let Some(diag) = check_number_style(self, source, name_str, &param.name_loc(), enforced_style, "variable") {
                    return vec![diag];
                }
            }
        }
        if let Some(param) = node.as_required_keyword_parameter_node() {
            let name = param.name().as_slice();
            let name_str = std::str::from_utf8(name).unwrap_or("");
            let bare = name_str.trim_end_matches(':');
            if !is_allowed(bare, &allowed_ids, &allowed_pats) {
                if let Some(diag) = check_number_style(self, source, bare, &param.name_loc(), enforced_style, "variable") {
                    return vec![diag];
                }
            }
        }
        if let Some(param) = node.as_optional_keyword_parameter_node() {
            let name = param.name().as_slice();
            let name_str = std::str::from_utf8(name).unwrap_or("");
            let bare = name_str.trim_end_matches(':');
            if !is_allowed(bare, &allowed_ids, &allowed_pats) {
                if let Some(diag) = check_number_style(self, source, bare, &param.name_loc(), enforced_style, "variable") {
                    return vec![diag];
                }
            }
        }

        Vec::new()
    }
}

fn is_allowed(name: &str, allowed_ids: &[String], allowed_pats: &[String]) -> bool {
    if allowed_ids.iter().any(|a| a == name) {
        return true;
    }
    for pattern in allowed_pats {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(name) {
                return true;
            }
        }
    }
    false
}

fn check_number_style(
    cop: &VariableNumber,
    source: &SourceFile,
    name: &str,
    loc: &ruby_prism::Location<'_>,
    enforced_style: &str,
    identifier_type: &str,
) -> Option<Diagnostic> {
    // Find if name contains digits
    let has_digit = name.bytes().any(|b| b.is_ascii_digit());
    if !has_digit {
        return None;
    }

    // Implicit params like _1, _2 are always allowed
    if name.starts_with('_') && name[1..].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    // Strip trailing `?` or `!` (method name suffixes) before checking
    let check_name = name.trim_end_matches(|c| c == '?' || c == '!');

    // RuboCop checks the END of the identifier against a format regex.
    // The key insight is only the TRAILING number pattern matters.
    //
    // normalcase:  /(?:\D|[^_\d]\d+|\A\d+)\z/ — trailing digits must NOT be preceded by _
    // snake_case:  /(?:\D|_\d+|\A\d+)\z/      — trailing digits MUST be preceded by _
    // non_integer: /(\D|\A\d+)\z/              — no trailing digits allowed
    let valid = match enforced_style {
        "normalcase" => is_valid_normalcase(check_name),
        "snake_case" => is_valid_snake_case(check_name),
        "non_integer" => is_valid_non_integer(check_name),
        _ => true,
    };

    if !valid {
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        return Some(cop.diagnostic(
            source,
            line,
            column,
            format!("Use {enforced_style} for {identifier_type} numbers."),
        ));
    }

    None
}

/// normalcase: /(?:\D|[^_\d]\d+|\A\d+)\z/
/// Valid if: ends with non-digit, OR ends with digits NOT preceded by _, OR is all digits
fn is_valid_normalcase(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return true;
    }
    let last = bytes[bytes.len() - 1];
    // Ends with non-digit → OK
    if !last.is_ascii_digit() {
        return true;
    }
    // Ends with digits. Find where the trailing digit run starts.
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1].is_ascii_digit() {
        i -= 1;
    }
    // If trailing digits span the whole string → OK (all digits)
    if i == 0 {
        return true;
    }
    // The character before the trailing digits must NOT be underscore
    bytes[i - 1] != b'_'
}

/// snake_case: /(?:\D|_\d+|\A\d+)\z/
/// Valid if: ends with non-digit, OR ends with digits preceded by _, OR is all digits
fn is_valid_snake_case(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return true;
    }
    let last = bytes[bytes.len() - 1];
    if !last.is_ascii_digit() {
        return true;
    }
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1].is_ascii_digit() {
        i -= 1;
    }
    if i == 0 {
        return true;
    }
    // The character before the trailing digits MUST be underscore
    bytes[i - 1] == b'_'
}

/// non_integer: /(\D|\A\d+)\z/
/// Valid if: ends with non-digit, OR is all digits
fn is_valid_non_integer(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return true;
    }
    let last = bytes[bytes.len() - 1];
    if !last.is_ascii_digit() {
        return true;
    }
    // Only valid if ALL digits
    bytes.iter().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(VariableNumber, "cops/naming/variable_number");
}
