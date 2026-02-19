use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE};

pub struct ContextWording;

const DEFAULT_PREFIXES: &[&str] = &["when", "with", "without"];

impl Cop for ContextWording {
    fn name(&self) -> &'static str {
        "RSpec/ContextWording"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = call.name().as_slice();
        if method != b"context" && method != b"shared_context" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Extract description text from string or interpolated string
        let content_str: String;
        if let Some(s) = arg_list[0].as_string_node() {
            let content = s.unescaped();
            content_str = match std::str::from_utf8(&content) {
                Ok(s) => s.to_string(),
                Err(_) => return Vec::new(),
            };
        } else if let Some(interp) = arg_list[0].as_interpolated_string_node() {
            // For interpolated strings, extract leading text before first interpolation
            let parts: Vec<_> = interp.parts().iter().collect();
            if let Some(first) = parts.first() {
                if let Some(s) = first.as_string_node() {
                    let text = s.unescaped();
                    content_str = match std::str::from_utf8(&text) {
                        Ok(s) => s.to_string(),
                        Err(_) => return Vec::new(),
                    };
                } else {
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }
        } else {
            return Vec::new();
        };

        // Config: AllowedPatterns — regex patterns to skip
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        // Check if description matches any allowed pattern
        if let Some(ref patterns) = allowed_patterns {
            for pat in patterns {
                if let Ok(re) = regex::Regex::new(pat) {
                    if re.is_match(&content_str) {
                        return Vec::new();
                    }
                }
            }
        }

        // Read Prefixes from config, fall back to defaults
        let config_prefixes = config.get_string_array("Prefixes");
        let prefixes: Vec<&str> = if let Some(ref arr) = config_prefixes {
            arr.iter().map(|s| s.as_str()).collect()
        } else {
            DEFAULT_PREFIXES.to_vec()
        };

        // Check if description starts with any allowed prefix followed by a word boundary
        for prefix in &prefixes {
            if content_str.starts_with(prefix) {
                let after = &content_str[prefix.len()..];
                if after.is_empty()
                    || after.starts_with(' ')
                    || after.starts_with(',')
                    || after.starts_with('\n')
                {
                    return Vec::new();
                }
            }
        }

        let prefix_display: Vec<String> =
            prefixes.iter().map(|p| format!("/^{p}\\b/")).collect();
        let loc = arg_list[0].location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Context description should match {}.",
                prefix_display.join(", ")
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ContextWording, "cops/rspec/context_wording");

    #[test]
    fn allowed_patterns_skips_matching_description() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("^if ".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"context 'if the user is logged in' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ContextWording, source, config);
        assert!(diags.is_empty(), "AllowedPatterns should skip matching descriptions");
    }

    #[test]
    fn allowed_patterns_does_not_skip_non_matching() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("^if ".into()),
                ]),
            )]),
            ..CopConfig::default()
        };
        let source = b"context 'the user is logged in' do\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&ContextWording, source, config);
        assert_eq!(diags.len(), 1);
    }
}
