use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::FLOAT_NODE;

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
    ) -> Vec<Diagnostic> {
        let float_node = match node.as_float_node() {
            Some(f) => f,
            None => return Vec::new(),
        };

        let loc = float_node.location();
        let src_bytes = loc.as_slice();
        let src_str = match std::str::from_utf8(src_bytes) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        // Only care about exponential notation (contains 'e' or 'E')
        let lower = src_str.to_lowercase();
        if !lower.contains('e') {
            return Vec::new();
        }

        // Strip leading minus for mantissa analysis
        let working = if lower.starts_with('-') { &lower[1..] } else { &lower };

        let parts: Vec<&str> = working.splitn(2, 'e').collect();
        if parts.len() != 2 {
            return Vec::new();
        }

        let mantissa_str = parts[0].replace('_', "");
        let exponent_str = parts[1].replace('_', "");

        let mantissa: f64 = match mantissa_str.parse() {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let exponent: i64 = match exponent_str.parse() {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let style = config.get_str("EnforcedStyle", "scientific");

        let (line, column) = source.offset_to_line_col(loc.start_offset());

        match style {
            "scientific" => {
                // Mantissa must be >= 1 and < 10
                let abs_mantissa = mantissa.abs();
                if abs_mantissa < 1.0 || abs_mantissa >= 10.0 {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use a mantissa >= 1 and < 10.".to_string(),
                    )];
                }
            }
            "engineering" => {
                // Exponent must be divisible by 3, mantissa >= 0.1 and < 1000
                let abs_mantissa = mantissa.abs();
                if exponent % 3 != 0 || abs_mantissa < 0.1 || abs_mantissa >= 1000.0 {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use an exponent divisible by 3 and a mantissa >= 0.1 and < 1000.".to_string(),
                    )];
                }
            }
            "integral" => {
                // Mantissa must be an integer without trailing zeros
                let has_decimal = mantissa_str.contains('.');
                let mantissa_int: i64 = if has_decimal {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use an integer as mantissa, without trailing zero.".to_string(),
                    )];
                } else {
                    match mantissa_str.parse() {
                        Ok(v) => v,
                        Err(_) => return Vec::new(),
                    }
                };
                if mantissa_int != 0 && mantissa_int % 10 == 0 {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Use an integer as mantissa, without trailing zero.".to_string(),
                    )];
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExponentialNotation, "cops/style/exponential_notation");
}
