use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct NumericLiterals;

impl Cop for NumericLiterals {
    fn name(&self) -> &'static str {
        "Style/NumericLiterals"
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

        let min_digits: usize = config
            .options
            .get("MinDigits")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(5);

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

        // Count actual digits (not underscores)
        let digit_count = digits_part.bytes().filter(|b| b.is_ascii_digit()).count();
        let has_underscores = digits_part.contains('_');

        if digit_count >= min_digits && !has_underscores {
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Use underscores(_) as thousands separator.".to_string(),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &NumericLiterals,
            include_bytes!("../../../testdata/cops/style/numeric_literals/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &NumericLiterals,
            include_bytes!("../../../testdata/cops/style/numeric_literals/no_offense.rb"),
        );
    }

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
}
