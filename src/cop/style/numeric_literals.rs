use crate::cop::node_type::{FLOAT_NODE, INTEGER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

const MSG: &str =
    "Use underscores(_) as thousands separator and separate every 3 digits with them.";

/// FP fix: implicit octal literals (leading `0` followed by digits, e.g. `00644`, `02744`)
/// were flagged for missing underscores. RuboCop exempts all non-decimal bases including
/// implicit octals. 199/404 FPs were from puppet's acceptance tests with file permission modes.
/// Other FP repos: jruby (98), ffi (30), peritor (27), natalie (20).
///
/// ## Investigation findings (2026-03-18)
///
/// ### FN root cause fixed:
/// - Float literals (e.g. `10000.0`, `123456.789`, `10000e10`) were not checked.
///   RuboCop's `on_float` handler extracts the integer part before `.` or `e/E`
///   and checks it the same way as integer literals. Added FLOAT_NODE support.
///
/// ## Investigation findings (2026-03-30)
///
/// ### FN root cause fixed:
/// - Default mode incorrectly accepted any decimal literal that already had underscores.
///   RuboCop still flags misgrouped underscores unless `Strict: false` specifically allows
///   a short trailing group (for example `10_000_00` or `123_456_789_00`).
/// - That missed Rails schema timestamps like `2021_12_12_143544` and cent-style literals
///   like `1099_99`, both of which contain a 4+ digit group and should still register.
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

fn has_bad_grouping(text: &str, strict: bool) -> bool {
    let groups: Vec<&str> = text.split('_').collect();

    if strict {
        return !is_correctly_grouped(text);
    }

    if groups.iter().any(|group| group.len() >= 4) {
        return true;
    }

    groups
        .iter()
        .skip(1)
        .take(groups.len().saturating_sub(2))
        .any(|group| matches!(group.len(), 1 | 2))
}

impl NumericLiterals {
    fn check_integer_part(
        &self,
        source: &SourceFile,
        loc: &ruby_prism::Location<'_>,
        int_part: &str,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let min_digits = config.get_usize("MinDigits", 5);
        let strict = config.get_bool("Strict", false);
        let allowed_numbers = config
            .get_string_array("AllowedNumbers")
            .unwrap_or_default();
        let allowed_patterns = config
            .get_string_array("AllowedPatterns")
            .unwrap_or_default();

        // Skip non-decimal literals (leading 0)
        if int_part.starts_with('0') {
            return;
        }

        // Get digits-only string (strip underscores)
        let int_str: String = int_part.chars().filter(|c| c.is_ascii_digit()).collect();

        // Check AllowedNumbers (compared as strings)
        if allowed_numbers.iter().any(|n| n == &int_str) {
            return;
        }

        // Check AllowedPatterns (regex-style match, anchored like RuboCop)
        if !allowed_patterns.is_empty() {
            for pattern in &allowed_patterns {
                if let Ok(re) = regex::Regex::new(&format!("\\A{}\\z", pattern)) {
                    if re.is_match(&int_str) {
                        return;
                    }
                } else if int_str.contains(pattern.as_str()) {
                    // Fallback to substring match if regex parsing fails
                    return;
                }
            }
        }

        let digit_count = int_str.len();
        let has_underscores = int_part.contains('_');

        if digit_count < min_digits {
            return;
        }

        if !has_underscores {
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
            return;
        }

        if has_bad_grouping(int_part, strict) {
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
        }
    }
}

impl Cop for NumericLiterals {
    fn name(&self) -> &'static str {
        "Style/NumericLiterals"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTEGER_NODE, FLOAT_NODE]
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
        // Get the source location and text for either integer or float nodes
        let loc = if let Some(int_node) = node.as_integer_node() {
            int_node.location()
        } else if let Some(float_node) = node.as_float_node() {
            float_node.location()
        } else {
            return;
        };

        let source_text = loc.as_slice();
        let text = std::str::from_utf8(source_text).unwrap_or("");

        // Extract the integer part: strip sign, split on e/E/., take the first part
        // This matches RuboCop's IntegerNode#integer_part behavior
        let unsigned = text
            .strip_prefix('-')
            .or(text.strip_prefix('+'))
            .unwrap_or(text);
        let int_part = unsigned.split(['e', 'E', '.']).next().unwrap_or(unsigned);

        self.check_integer_part(source, &loc, int_part, config, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(NumericLiterals, "cops/style/numeric_literals");

    #[test]
    fn config_min_digits_3() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("MinDigits".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 3-digit number without underscores should trigger with MinDigits:3
        let source = b"x = 100\n";
        let diags = run_cop_full_with_config(&NumericLiterals, source, config.clone());
        assert!(
            !diags.is_empty(),
            "Should fire with MinDigits:3 on 3-digit number"
        );

        // 2-digit number should NOT trigger
        let source2 = b"x = 99\n";
        let diags2 = run_cop_full_with_config(&NumericLiterals, source2, config);
        assert!(
            diags2.is_empty(),
            "Should not fire on 2-digit number with MinDigits:3"
        );
    }

    #[test]
    fn strict_mode_flags_bad_grouping() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

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
        assert!(
            diags2.is_empty(),
            "Correctly grouped number should pass strict mode"
        );
    }

    #[test]
    fn allowed_numbers_exempts() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedNumbers".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("10000".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"x = 10000\n";
        let diags = run_cop_full_with_config(&NumericLiterals, source, config);
        assert!(diags.is_empty(), "AllowedNumbers should exempt 10000");
    }
}
