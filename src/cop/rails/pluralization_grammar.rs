use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct PluralizationGrammar;

const SINGULAR_TO_PLURAL: &[(&[u8], &str)] = &[
    (b"second", "seconds"),
    (b"minute", "minutes"),
    (b"hour", "hours"),
    (b"day", "days"),
    (b"week", "weeks"),
    (b"fortnight", "fortnights"),
    (b"month", "months"),
    (b"year", "years"),
    (b"byte", "bytes"),
    (b"kilobyte", "kilobytes"),
    (b"megabyte", "megabytes"),
    (b"gigabyte", "gigabytes"),
    (b"terabyte", "terabytes"),
    (b"petabyte", "petabytes"),
    (b"exabyte", "exabytes"),
    (b"zettabyte", "zettabytes"),
];

const PLURAL_TO_SINGULAR: &[(&[u8], &str)] = &[
    (b"seconds", "second"),
    (b"minutes", "minute"),
    (b"hours", "hour"),
    (b"days", "day"),
    (b"weeks", "week"),
    (b"fortnights", "fortnight"),
    (b"months", "month"),
    (b"years", "year"),
    (b"bytes", "byte"),
    (b"kilobytes", "kilobyte"),
    (b"megabytes", "megabyte"),
    (b"gigabytes", "gigabyte"),
    (b"terabytes", "terabyte"),
    (b"petabytes", "petabyte"),
    (b"exabytes", "exabyte"),
    (b"zettabytes", "zettabyte"),
];

fn is_duration_method(name: &[u8]) -> bool {
    SINGULAR_TO_PLURAL.iter().any(|(s, _)| *s == name)
        || PLURAL_TO_SINGULAR.iter().any(|(p, _)| *p == name)
}

fn is_plural(name: &[u8]) -> bool {
    PLURAL_TO_SINGULAR.iter().any(|(p, _)| *p == name)
}

fn correct_method(name: &[u8]) -> Option<&'static str> {
    if let Some((_, plural)) = SINGULAR_TO_PLURAL.iter().find(|(s, _)| *s == name) {
        return Some(plural);
    }
    if let Some((_, singular)) = PLURAL_TO_SINGULAR.iter().find(|(p, _)| *p == name) {
        return Some(singular);
    }
    None
}

impl Cop for PluralizationGrammar {
    fn name(&self) -> &'static str {
        "Rails/PluralizationGrammar"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        if !is_duration_method(method_name) {
            return Vec::new();
        }

        // Receiver must be a numeric literal
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let number = if let Some(int_node) = receiver.as_integer_node() {
            // Extract the integer value from source
            let loc = int_node.location();
            let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let text_str = std::str::from_utf8(text).unwrap_or("0");
            // Remove underscores for parsing
            let clean: String = text_str.chars().filter(|c| *c != '_').collect();
            clean.parse::<i64>().ok()
        } else if let Some(float_node) = receiver.as_float_node() {
            let loc = float_node.location();
            let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let text_str = std::str::from_utf8(text).unwrap_or("0");
            let clean: String = text_str.chars().filter(|c| *c != '_').collect();
            // Only treat as integer if the float is a whole number (e.g. 1.0, -1.0)
            // Fractional values like 1.5 are not singular or plural in the integer sense
            clean.parse::<f64>().ok().and_then(|f| {
                if f == f.trunc() {
                    Some(f as i64)
                } else {
                    None
                }
            })
        } else {
            return Vec::new();
        };

        let number = match number {
            Some(n) => n,
            None => return Vec::new(),
        };

        let is_singular_number = number.abs() == 1;
        let is_plural_method = is_plural(method_name);

        // Offense: singular number with plural method, or plural number with singular method
        let should_flag = (is_singular_number && is_plural_method)
            || (!is_singular_number && !is_plural_method);

        if !should_flag {
            return Vec::new();
        }

        let correct = match correct_method(method_name) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{number}.{correct}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PluralizationGrammar, "cops/rails/pluralization_grammar");
}
