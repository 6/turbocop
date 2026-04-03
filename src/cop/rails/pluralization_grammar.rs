use crate::cop::shared::node_type::{CALL_NODE, FLOAT_NODE, INTEGER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/PluralizationGrammar — checks that numeric literals use grammatically
/// correct ActiveSupport duration/byte methods (singular for 1/-1, plural otherwise).
///
/// ## Investigation (2026-03-15)
///
/// **Root cause of 40 FNs:** All false negatives were fractional float literals
/// (e.g., `0.1.second`, `0.5.hour`, `1.5.day`) with singular method names.
/// The old implementation discarded non-integer floats (where `f != f.trunc()`),
/// so these expressions were never checked. RuboCop treats ALL float literals
/// as valid receivers — `0.1` is not singular (abs != 1), so it requires the
/// plural form (`0.1.seconds`).
///
/// **Fix:** Accept all float values, not just whole-number floats. The singularity
/// check is `f.abs() == 1.0`, matching RuboCop's `number.abs == 1` behavior.
/// Non-integer floats like 0.1, 0.5, 1.5 are always plural.
///
/// **FP: 0, FN: 0 after fix.**
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FLOAT_NODE, INTEGER_NODE]
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
        if !is_duration_method(method_name) {
            return;
        }

        // Receiver must be a numeric literal
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // RuboCop accepts any int or float literal as a receiver.
        // Singular means abs(value) == 1 (e.g., 1, -1, 1.0, -1.0).
        // All other numeric values (including fractional floats like 0.1, 1.5)
        // are considered plural.
        let (is_singular_number, number_text) = if let Some(int_node) = receiver.as_integer_node() {
            let loc = int_node.location();
            let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let text_str = std::str::from_utf8(text).unwrap_or("0");
            let clean: String = text_str.chars().filter(|c| *c != '_').collect();
            match clean.parse::<i64>() {
                Ok(n) => (n.abs() == 1, text_str.to_string()),
                Err(_) => return,
            }
        } else if let Some(float_node) = receiver.as_float_node() {
            let loc = float_node.location();
            let text = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let text_str = std::str::from_utf8(text).unwrap_or("0");
            let clean: String = text_str.chars().filter(|c| *c != '_').collect();
            match clean.parse::<f64>() {
                Ok(f) => (f.abs() == 1.0, text_str.to_string()),
                Err(_) => return,
            }
        } else {
            return;
        };
        let is_plural_method = is_plural(method_name);

        // Offense: singular number with plural method, or plural number with singular method
        let should_flag =
            (is_singular_number && is_plural_method) || (!is_singular_number && !is_plural_method);

        if !should_flag {
            return;
        }

        let correct = match correct_method(method_name) {
            Some(c) => c,
            None => return,
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{number_text}.{correct}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PluralizationGrammar, "cops/rails/pluralization_grammar");
}
