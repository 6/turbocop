use crate::cop::shared::node_type::FLOAT_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use regex::Regex;
use std::sync::LazyLock;

static SCIENTIFIC_MANTISSA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?[1-9](\.\d*[0-9])?$").unwrap());
static ENGINEERING_EXPONENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-?\d+$").unwrap());
static ENGINEERING_LARGE_MANTISSA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?\d{4}").unwrap());
static ENGINEERING_LEADING_ZERO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?0\d").unwrap());
static ENGINEERING_SMALL_MANTISSA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?0.0").unwrap());
static INTEGRAL_MANTISSA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?[1-9](\d*[1-9])?$").unwrap());

/// ## Corpus investigation (2026-03-25)
///
/// Corpus oracle reported FP=149, FN=4.
///
/// FP=149: RuboCop only checks for lowercase `e` in exponential notation
/// (`node.source['e']`). Uppercase `E` (e.g., `0.22E1`) is ignored. Nitrocop
/// was lowercasing first and matching both. Fix: check for lowercase `e` only.
///
/// ## Corpus investigation (2026-03-26)
///
/// FN=4 in `natalie-lang/natalie`: RuboCop validates the raw mantissa text with
/// regexes and rejects a leading `+` sign in scientific notation. Prism includes
/// that `+` in the `FloatNode` source (`+2.5e20`, `+2.5e200`), but nitrocop was
/// parsing numerically and treating those sources the same as `2.5e20`. Fix:
/// mirror RuboCop's source-pattern checks for all styles instead of normalizing
/// to numeric ranges.
pub struct ExponentialNotation;

impl Cop for ExponentialNotation {
    fn name(&self) -> &'static str {
        "Style/ExponentialNotation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[FLOAT_NODE]
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
        let float_node = match node.as_float_node() {
            Some(f) => f,
            None => return,
        };

        let loc = float_node.location();
        let src_bytes = loc.as_slice();
        let src_str = match std::str::from_utf8(src_bytes) {
            Ok(s) => s,
            Err(_) => return,
        };

        // RuboCop only checks for lowercase 'e' — uppercase 'E' (e.g., 0.22E1) is ignored
        if !src_str.contains('e') {
            return;
        }

        let (mantissa, exponent) = match src_str.split_once('e') {
            Some(parts) => parts,
            None => return,
        };

        let style = config.get_str("EnforcedStyle", "scientific");
        let message = match style {
            "scientific" => {
                if SCIENTIFIC_MANTISSA_RE.is_match(mantissa) {
                    return;
                }
                "Use a mantissa >= 1 and < 10."
            }
            "engineering" => {
                let exponent_ok = ENGINEERING_EXPONENT_RE.is_match(exponent);
                if exponent_ok
                    && exponent_divisible_by_three(exponent)
                    && !ENGINEERING_LARGE_MANTISSA_RE.is_match(mantissa)
                    && !ENGINEERING_LEADING_ZERO_RE.is_match(mantissa)
                    && !ENGINEERING_SMALL_MANTISSA_RE.is_match(mantissa)
                {
                    return;
                }
                "Use an exponent divisible by 3 and a mantissa >= 0.1 and < 1000."
            }
            "integral" => {
                if INTEGRAL_MANTISSA_RE.is_match(mantissa) {
                    return;
                }
                "Use an integer as mantissa, without trailing zero."
            }
            _ => return,
        };

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
    }
}

fn exponent_divisible_by_three(exponent: &str) -> bool {
    let digits = exponent.strip_prefix('-').unwrap_or(exponent);
    if digits.is_empty() {
        return false;
    }

    digits
        .bytes()
        .fold(0u8, |acc, byte| (acc * 10 + (byte - b'0')) % 3)
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExponentialNotation, "cops/style/exponential_notation");
}
