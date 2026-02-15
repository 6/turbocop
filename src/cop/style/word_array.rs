use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct WordArray;

impl Cop for WordArray {
    fn name(&self) -> &'static str {
        "Style/WordArray"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Must have `[` opening (not %w or %W)
        let opening = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        if opening.as_slice() != b"[" {
            return Vec::new();
        }

        let elements = array_node.elements();
        let min_size = config.get_usize("MinSize", 2);
        let enforced_style = config.get_str("EnforcedStyle", "percent");
        // WordRegex: custom regex for what constitutes a "word". When set, only
        // strings matching this regex are considered words eligible for %w.
        // Read for completeness; basic regex support is limited.
        let word_regex = config.get_str("WordRegex", "");

        // "brackets" style: never flag bracket arrays
        if enforced_style == "brackets" {
            return Vec::new();
        }

        if elements.len() < min_size {
            return Vec::new();
        }

        // All elements must be simple string nodes
        for elem in elements.iter() {
            let string_node = match elem.as_string_node() {
                Some(s) => s,
                None => return Vec::new(),
            };

            // Must have an opening quote (not a bare string)
            if string_node.opening_loc().is_none() {
                return Vec::new();
            }

            // Content must not contain spaces
            let content = string_node.content_loc().as_slice();
            if content.contains(&b' ') {
                return Vec::new();
            }

            // Must not have escape sequences (backslash in content)
            if content.contains(&b'\\') {
                return Vec::new();
            }

            // WordRegex: if set, check that content matches (simple contains check)
            if !word_regex.is_empty() {
                let content_str = std::str::from_utf8(content).unwrap_or("");
                // Simple check: if WordRegex looks like a restrictive pattern,
                // only flag if content matches basic word chars
                if word_regex.contains("\\A") || word_regex.contains("\\w") {
                    if !content_str.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        return Vec::new();
                    }
                }
            }
        }

        let (line, column) = source.offset_to_line_col(opening.start_offset());
        vec![self.diagnostic(source, line, column, "Use `%w` or `%W` for an array of words.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(WordArray, "cops/style/word_array");

    #[test]
    fn config_min_size_5() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("MinSize".into(), serde_yml::Value::Number(5.into()))]),
            ..CopConfig::default()
        };
        // 5 elements should trigger with MinSize:5
        let source = b"x = ['a', 'b', 'c', 'd', 'e']\n";
        let diags = run_cop_full_with_config(&WordArray, source, config.clone());
        assert!(!diags.is_empty(), "Should fire with MinSize:5 on 5-element word array");

        // 4 elements should NOT trigger
        let source2 = b"x = ['a', 'b', 'c', 'd']\n";
        let diags2 = run_cop_full_with_config(&WordArray, source2, config);
        assert!(diags2.is_empty(), "Should not fire on 4-element word array with MinSize:5");
    }

    #[test]
    fn brackets_style_allows_bracket_arrays() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("brackets".into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"x = ['a', 'b', 'c']\n";
        let diags = run_cop_full_with_config(&WordArray, source, config);
        assert!(diags.is_empty(), "Should not flag brackets with brackets style");
    }
}
