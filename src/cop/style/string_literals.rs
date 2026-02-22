use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct StringLiterals;

impl Cop for StringLiterals {
    fn name(&self) -> &'static str {
        "Style/StringLiterals"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "single_quotes").to_string();
        let consistent_multiline = config.get_bool("ConsistentQuotesInMultiline", false);

        let mut visitor = StringLiteralsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            enforced_style,
            consistent_multiline,
            in_interpolation: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct StringLiteralsVisitor<'a> {
    cop: &'a StringLiterals,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    enforced_style: String,
    consistent_multiline: bool,
    in_interpolation: bool,
}

impl<'pr> Visit<'pr> for StringLiteralsVisitor<'_> {
    fn visit_embedded_statements_node(&mut self, node: &ruby_prism::EmbeddedStatementsNode<'pr>) {
        let was = self.in_interpolation;
        self.in_interpolation = true;
        ruby_prism::visit_embedded_statements_node(self, node);
        self.in_interpolation = was;
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        let opening = match node.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        let opening_byte = opening.as_slice().first().copied().unwrap_or(0);

        // Skip %q, %Q, heredocs, ? prefix
        if matches!(opening_byte, b'%' | b'<' | b'?') {
            return;
        }

        let content = node.content_loc().as_slice();

        // When ConsistentQuotesInMultiline is enabled, skip multiline strings —
        // these should be checked for consistency as a group (not individually)
        if self.consistent_multiline && content.contains(&b'\n') {
            return;
        }

        match self.enforced_style.as_str() {
            "single_quotes" => {
                if opening_byte == b'"' {
                    // Skip multi-line strings — RuboCop doesn't flag these
                    if content.contains(&b'\n') {
                        return;
                    }
                    // Check if single quotes can be used:
                    // - No single quotes in content
                    // - No escape sequences (no backslash in content)
                    if !content.contains(&b'\'') && !needs_double_quotes(content) {
                        let (line, column) = self.source.offset_to_line_col(opening.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(self.source, line, column, "Prefer single-quoted strings when you don't need string interpolation or special symbols.".to_string()));
                    }
                }
            }
            "double_quotes" => {
                if opening_byte == b'\'' {
                    // Skip if the content contains double quotes — converting would
                    // require escaping, so the single-quoted form is preferred.
                    if content.contains(&b'"') {
                        return;
                    }
                    // Skip if the content contains backslashes — in single-quoted
                    // strings they're literal, but in double-quoted strings they
                    // become escape sequences (changing the string's meaning).
                    if content.contains(&b'\\') {
                        return;
                    }
                    // Skip if content contains #{ — in double quotes this would
                    // become interpolation, changing the string's meaning.
                    if content.windows(2).any(|w| w == b"#{") {
                        return;
                    }
                    // Skip multi-line strings — RuboCop doesn't flag these
                    // in the per-string StringLiterals check.
                    if content.contains(&b'\n') {
                        return;
                    }
                    // Skip if this string is inside a #{ } interpolation context —
                    // converting to double quotes would need escaping inside the
                    // enclosing double-quoted string.
                    if self.in_interpolation {
                        return;
                    }
                    let (line, column) = self.source.offset_to_line_col(opening.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(self.source, line, column, "Prefer double-quoted strings unless you need single quotes within your string.".to_string()));
                }
            }
            _ => {}
        }
    }
}

/// Check if a double-quoted string's raw source content contains escape
/// sequences that require double quotes, matching RuboCop's
/// `double_quotes_required?` logic. A backslash followed by any character
/// that is NOT `\` or `"` is considered to require double quotes — this is
/// conservative but prevents visual changes to escape-like sequences such
/// as `\g`, `\p`, etc.
fn needs_double_quotes(content: &[u8]) -> bool {
    let mut i = 0;
    while i < content.len() {
        if content[i] == b'\\' && i + 1 < content.len() {
            let next = content[i + 1];
            // \\ and \" are safe to convert (they become literal chars in single quotes too)
            if next == b'\\' || next == b'"' {
                i += 2;
                continue;
            }
            // Any other \X pattern requires double quotes
            return true;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(StringLiterals, "cops/style/string_literals");

    #[test]
    fn config_double_quotes() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("double_quotes".into()),
            )]),
            ..CopConfig::default()
        };
        // Single-quoted string should trigger with double_quotes style
        let source = b"x = 'hello'\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire with EnforcedStyle:double_quotes on single-quoted string"
        );
        assert!(diags[0].message.contains("double-quoted"));
    }

    #[test]
    fn double_quotes_skips_inside_interpolation() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("double_quotes".into()),
            )]),
            ..CopConfig::default()
        };
        // Single-quoted string inside interpolation should NOT be flagged
        let source = b"x = \"hello #{env['KEY']}\"\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag single-quoted string inside interpolation: {:?}",
            diags
        );
    }

    #[test]
    fn double_quotes_skips_string_containing_double_quotes() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("double_quotes".into()),
            )]),
            ..CopConfig::default()
        };
        // Single-quoted string containing " should NOT be flagged
        let source = b"x = 'say \"hello\"'\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag single-quoted string with double quotes inside"
        );
    }

    #[test]
    fn double_quotes_skips_hash_brace_content() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("double_quotes".into()),
            )]),
            ..CopConfig::default()
        };
        // Single-quoted string containing #{ should NOT be flagged —
        // converting to double quotes would make it interpolation
        let source = b"x = '#{'\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag single-quoted string containing #{{: {:?}",
            diags
        );
    }

    #[test]
    fn double_quotes_skips_multiline_strings() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("double_quotes".into()),
            )]),
            ..CopConfig::default()
        };
        // Multi-line single-quoted string should NOT be flagged
        let source = b"x = '\n  hello\n  world\n'\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag multi-line single-quoted string: {:?}",
            diags
        );
    }

    #[test]
    fn double_quotes_flags_string_inside_hash() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("double_quotes".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo(custom_attributes: { tenant_id: 'different' })\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag single-quoted string inside hash arg: {:?}",
            diags
        );
    }

    #[test]
    fn double_quotes_flags_string_after_earlier_interpolation() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("double_quotes".into()),
            )]),
            ..CopConfig::default()
        };
        // Earlier in the file there's a string with interpolation, and later a
        // single-quoted string inside a hash literal. The hash braces should NOT
        // be confused with interpolation braces.
        let source =
            b"x = \"hello #{world}\"\nfoo(custom_attributes: { tenant_id: 'different' })\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag 'different' even with earlier interpolation in file: {:?}",
            diags
        );
    }

    #[test]
    fn consistent_multiline_skips_multiline_strings() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "ConsistentQuotesInMultiline".into(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        // Multiline string with double quotes should not be flagged when ConsistentQuotesInMultiline is true
        let source = b"x = \"hello\\nworld\"\n";
        let diags = run_cop_full_with_config(&StringLiterals, source, config);
        // The string contains \n (escape), so single quotes can't be used — shouldn't fire anyway
        assert!(diags.is_empty());
    }
}
