use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::INTEGER_NODE;

pub struct NumericLiterals;

/// Check if a numeric string has underscores at every 3-digit grouping from the right.
/// E.g., "1_000_000" is correct, "10_000_00" is not.
fn is_correctly_grouped(text: &str) -> bool {
    // Split on underscores and check groups
    let groups: Vec<&str> = text.split('_').collect();
    if groups.len() < 2 {
        return false;
    }
    // First group can be 1-3 digits, remaining groups must be exactly 3 digits
    for (i, group) in groups.iter().enumerate() {
        if i == 0 {
            if group.is_empty() || group.len() > 3 || !group.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
        } else if group.len() != 3 || !group.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
    }
    true
}

impl Cop for NumericLiterals {
    fn name(&self) -> &'static str {
        "Style/NumericLiterals"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTEGER_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let int_node = match node.as_integer_node() {
            Some(i) => i,
            None => return Vec::new(),
        };

        let loc = int_node.location();
        let source_text = loc.as_slice();

        let min_digits = config.get_usize("MinDigits", 5);
        let strict = config.get_bool("Strict", false);
        let allowed_numbers = config.get_string_array("AllowedNumbers").unwrap_or_default();
        let allowed_patterns = config.get_string_array("AllowedPatterns").unwrap_or_default();

        let text = std::str::from_utf8(source_text).unwrap_or("");

        // Skip non-decimal literals (0x, 0b, 0o, 0d prefixed)
        if text.starts_with("0x")
            || text.starts_with("0X")
            || text.starts_with("0b")
            || text.starts_with("0B")
            || text.starts_with("0o")
            || text.starts_with("0O")
            || text.starts_with("0d")
            || text.starts_with("0D")
        {
            return Vec::new();
        }

        // Strip leading minus sign if present
        let digits_part = if text.starts_with('-') {
            &text[1..]
        } else {
            text
        };

        // Get the integer-only portion (digits and underscores, no sign)
        let int_str: String = digits_part.chars().filter(|c| c.is_ascii_digit()).collect();

        // Check AllowedNumbers (compared as strings)
        if allowed_numbers.iter().any(|n| n == &int_str) {
            return Vec::new();
        }

        // Check AllowedPatterns (simple substring match, similar to RuboCop)
        if !allowed_patterns.is_empty() {
            for pattern in &allowed_patterns {
                if int_str.contains(pattern.as_str()) {
                    return Vec::new();
                }
            }
        }

        // Count actual digits (not underscores)
        let digit_count = int_str.len();
        let has_underscores = digits_part.contains('_');

        if digit_count < min_digits {
            return Vec::new();
        }

        if !has_underscores {
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(source, line, column, "Use underscores(_) as thousands separator.".to_string())];
        }

        // Strict mode: check that underscores are at correct every-3-digit positions
        if strict && !is_correctly_grouped(digits_part) {
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(source, line, column, "Use underscores(_) as thousands separator.".to_string())];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(NumericLiterals, "cops/style/numeric_literals");

    #[test]
    fn config_min_digits_3() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("MinDigits".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 3-digit number without underscores should trigger with MinDigits:3
        let source = b"x = 100\n";
        let diags = run_cop_full_with_config(&NumericLiterals, source, config.clone());
        assert!(!diags.is_empty(), "Should fire with MinDigits:3 on 3-digit number");

        // 2-digit number should NOT trigger
        let source2 = b"x = 99\n";
        let diags2 = run_cop_full_with_config(&NumericLiterals, source2, config);
        assert!(diags2.is_empty(), "Should not fire on 2-digit number with MinDigits:3");
    }

    #[test]
    fn strict_mode_flags_bad_grouping() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("Strict".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        // 10_000_00 has underscores but not at correct 3-digit grouping
        let source = b"x = 10_000_00\n";
        let diags = run_cop_full_with_config(&NumericLiterals, source, config.clone());
        assert_eq!(diags.len(), 1, "Strict mode should flag bad grouping");

        // 1_000_000 is correctly grouped
        let source2 = b"x = 1_000_000\n";
        let diags2 = run_cop_full_with_config(&NumericLiterals, source2, config);
        assert!(diags2.is_empty(), "Correctly grouped number should pass strict mode");
    }

    #[test]
    fn allowed_numbers_exempts() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowedNumbers".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("10000".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        let source = b"x = 10000\n";
        let diags = run_cop_full_with_config(&NumericLiterals, source, config);
        assert!(diags.is_empty(), "AllowedNumbers should exempt 10000");
    }
}
