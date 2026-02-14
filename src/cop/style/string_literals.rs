use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct StringLiterals;

impl Cop for StringLiterals {
    fn name(&self) -> &'static str {
        "Style/StringLiterals"
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

        let content = string_node.content_loc().as_slice();

        match enforced_style {
            "single_quotes" => {
                if opening_byte == b'"' {
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
}
