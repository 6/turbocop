use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FilePath;

impl Cop for FilePath {
    fn name(&self) -> &'static str {
        "Rails/FilePath"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "slashes");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"join" {
            return Vec::new();
        }

        // Receiver should be a call to `root` on `Rails`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let root_call = match recv.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if root_call.name().as_slice() != b"root" {
            return Vec::new();
        }
        let rails_recv = match root_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        // Handle both ConstantReadNode (Rails) and ConstantPathNode (::Rails)
        if util::constant_name(&rails_recv) != Some(b"Rails") {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();

        // All args should be strings
        let all_strings = arg_list.iter().all(|a| a.as_string_node().is_some());
        if !all_strings {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        match style {
            "arguments" => {
                // Flag single-arg calls that contain a slash separator
                if arg_list.len() == 1 {
                    if let Some(s) = arg_list[0].as_string_node() {
                        let val = s.unescaped();
                        if val.windows(1).any(|w| w == b"/") {
                            return vec![self.diagnostic(
                                source,
                                line,
                                column,
                                "Use `Rails.root.join('path', 'to')` with separate arguments.".to_string(),
                            )];
                        }
                    }
                }
                Vec::new()
            }
            _ => {
                // "slashes" (default): flag multi-arg join calls
                if arg_list.len() < 2 {
                    return Vec::new();
                }
                vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `Rails.root.join('app/models')` with a single path string.".to_string(),
                )]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FilePath, "cops/rails/file_path");

    #[test]
    fn arguments_style_flags_slash_in_single_arg() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("arguments".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"Rails.root.join('app/models')\n";
        let diags = run_cop_full_with_config(&FilePath, source, config);
        assert!(!diags.is_empty(), "arguments style should flag slash-separated path");
    }

    #[test]
    fn arguments_style_allows_multi_arg() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("arguments".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"Rails.root.join('app', 'models')\n";
        assert_cop_no_offenses_full_with_config(&FilePath, source, config);
    }
}
