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
        let _check_method_names = config.get_bool("CheckMethodNames", true);
        let _check_symbols = config.get_bool("CheckSymbols", true);
        let allowed = config.get_string_array("AllowedIdentifiers");
        let _allowed_patterns = config.get_string_array("AllowedPatterns");

        let allowed_ids: Vec<String> = allowed.unwrap_or_else(|| {
            DEFAULT_ALLOWED.iter().map(|s| s.to_string()).collect()
        });

        // Check local variable writes
        if let Some(lvar) = node.as_local_variable_write_node() {
            let name = lvar.name().as_slice();
            let name_str = std::str::from_utf8(name).unwrap_or("");
            if !allowed_ids.iter().any(|a| a == name_str) {
                if let Some(diag) = check_number_style(self, source, name_str, &lvar.name_loc(), enforced_style) {
                    return vec![diag];
                }
            }
        }

        Vec::new()
    }
}

fn check_number_style(
    cop: &VariableNumber,
    source: &SourceFile,
    name: &str,
    loc: &ruby_prism::Location<'_>,
    enforced_style: &str,
) -> Option<Diagnostic> {
    // Find if name contains digits
    let has_digit = name.bytes().any(|b| b.is_ascii_digit());
    if !has_digit {
        return None;
    }

    let violation = match enforced_style {
        "normalcase" => {
            // normalcase: digits should not be preceded by underscore
            // e.g., foo1 is OK, foo_1 is not
            has_underscore_before_digit(name)
        }
        "snake_case" => {
            // snake_case: digits must be preceded by underscore
            // e.g., foo_1 is OK, foo1 is not
            !has_underscore_before_digit(name) && has_digit_after_alpha(name)
        }
        "non_integer" => {
            // non_integer: no digits allowed at all
            has_digit
        }
        _ => false,
    };

    if violation {
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        return Some(cop.diagnostic(
            source,
            line,
            column,
            format!("Use {enforced_style} for variable numbers."),
        ));
    }

    None
}

fn has_underscore_before_digit(name: &str) -> bool {
    let bytes = name.as_bytes();
    for i in 1..bytes.len() {
        if bytes[i].is_ascii_digit() && bytes[i - 1] == b'_' {
            return true;
        }
    }
    false
}

fn has_digit_after_alpha(name: &str) -> bool {
    let bytes = name.as_bytes();
    for i in 1..bytes.len() {
        if bytes[i].is_ascii_digit() && bytes[i - 1].is_ascii_alphabetic() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(VariableNumber, "cops/naming/variable_number");
}
