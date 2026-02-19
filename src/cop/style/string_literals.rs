use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::STRING_NODE;

pub struct StringLiterals;

/// Check if the byte at `offset` is inside a `#{ }` interpolation context
/// by scanning backward. Tracks brace depth to handle nested `{ }` blocks
/// (from hashes, method blocks, etc.) that appear inside the interpolation.
fn is_inside_interpolation(source_bytes: &[u8], offset: usize) -> bool {
    let mut depth: i32 = 0;
    let mut i = offset;
    while i > 0 {
        i -= 1;
        match source_bytes[i] {
            b'}' => depth += 1,
            b'{' => {
                depth -= 1;
                if depth < 0 && i > 0 && source_bytes[i - 1] == b'#' {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

impl Cop for StringLiterals {
    fn name(&self) -> &'static str {
        "Style/StringLiterals"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let string_node = match node.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let opening = match string_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let opening_byte = opening.as_slice().first().copied().unwrap_or(0);

        // Skip %q, %Q, heredocs, ? prefix
        if matches!(opening_byte, b'%' | b'<' | b'?') {
            return Vec::new();
        }

        let enforced_style = config.get_str("EnforcedStyle", "single_quotes");
        // ConsistentQuotesInMultiline: when true, skip flagging individual strings
        // that contain newlines (multiline strings), deferring to consistency checking.
        let consistent_multiline = config.get_bool("ConsistentQuotesInMultiline", false);

        let content = string_node.content_loc().as_slice();

        // When ConsistentQuotesInMultiline is enabled, skip multiline strings —
        // these should be checked for consistency as a group (not individually)
        if consistent_multiline && content.contains(&b'\n') {
            return Vec::new();
        }

        match enforced_style {
            "single_quotes" => {
                if opening_byte == b'"' {
                    // Skip multi-line strings — RuboCop doesn't flag these
                    if content.contains(&b'\n') {
                        return Vec::new();
                    }
                    // Check if single quotes can be used:
                    // - No single quotes in content
                    // - No escape sequences (no backslash in content)
                    if !content.contains(&b'\'') && !content.contains(&b'\\') {
                        let (line, column) = source.offset_to_line_col(opening.start_offset());
                        return vec![self.diagnostic(source, line, column, "Prefer single-quoted strings when you don't need string interpolation or special symbols.".to_string())];
                    }
                }
            }
            "double_quotes" => {
                if opening_byte == b'\'' {
                    // Skip if the content contains double quotes — converting would
                    // require escaping, so the single-quoted form is preferred.
                    if content.contains(&b'"') {
                        return Vec::new();
                    }
                    // Skip if the content contains backslashes — in single-quoted
                    // strings they're literal, but in double-quoted strings they
                    // become escape sequences (changing the string's meaning).
                    if content.contains(&b'\\') {
                        return Vec::new();
                    }
                    // Skip if content contains #{ — in double quotes this would
                    // become interpolation, changing the string's meaning.
                    if content.windows(2).any(|w| w == b"#{") {
                        return Vec::new();
                    }
                    // Skip multi-line strings — RuboCop doesn't flag these
                    // in the per-string StringLiterals check.
                    if content.contains(&b'\n') {
                        return Vec::new();
                    }
                    // Skip if this string is inside a #{ } interpolation context —
                    // converting to double quotes would need escaping inside the
                    // enclosing double-quoted string.
                    if is_inside_interpolation(source.as_bytes(), opening.start_offset()) {
                        return Vec::new();
                    }
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    return vec![self.diagnostic(source, line, column, "Prefer double-quoted strings unless you need single quotes within your string.".to_string())];
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

    crate::cop_fixture_tests!(StringLiterals, "cops/style/string_literals");

    #[test]
    fn config_double_quotes() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("double_quotes".into())),
            ]),
            ..CopConfig::default()
        };
        // Single-quoted string should trigger with double_quotes style
        let source = b"x = 'hello'\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert!(!diags.is_empty(), "Should fire with EnforcedStyle:double_quotes on single-quoted string");
        assert!(diags[0].message.contains("double-quoted"));
    }

    #[test]
    fn double_quotes_skips_inside_interpolation() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("double_quotes".into())),
            ]),
            ..CopConfig::default()
        };
        // Single-quoted string inside interpolation should NOT be flagged
        let source = b"x = \"hello #{env['KEY']}\"\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert!(diags.is_empty(), "Should not flag single-quoted string inside interpolation: {:?}", diags);
    }

    #[test]
    fn double_quotes_skips_string_containing_double_quotes() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("double_quotes".into())),
            ]),
            ..CopConfig::default()
        };
        // Single-quoted string containing " should NOT be flagged
        let source = b"x = 'say \"hello\"'\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert!(diags.is_empty(), "Should not flag single-quoted string with double quotes inside");
    }

    #[test]
    fn double_quotes_skips_hash_brace_content() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("double_quotes".into())),
            ]),
            ..CopConfig::default()
        };
        // Single-quoted string containing #{ should NOT be flagged —
        // converting to double quotes would make it interpolation
        let source = b"x = '#{'\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert!(diags.is_empty(), "Should not flag single-quoted string containing #{{: {:?}", diags);
    }

    #[test]
    fn double_quotes_skips_multiline_strings() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("double_quotes".into())),
            ]),
            ..CopConfig::default()
        };
        // Multi-line single-quoted string should NOT be flagged
        let source = b"x = '\n  hello\n  world\n'\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert!(diags.is_empty(), "Should not flag multi-line single-quoted string: {:?}", diags);
    }

    #[test]
    fn consistent_multiline_skips_multiline_strings() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("ConsistentQuotesInMultiline".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        // Multiline string with double quotes should not be flagged when ConsistentQuotesInMultiline is true
        let source = b"x = \"hello\\nworld\"\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        // The string contains \n (escape), so single quotes can't be used — shouldn't fire anyway
        assert!(diags.is_empty());
    }
}
