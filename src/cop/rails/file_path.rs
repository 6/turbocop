use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, EMBEDDED_STATEMENTS_NODE,
    INTERPOLATED_STRING_NODE, LOCAL_VARIABLE_READ_NODE, STRING_NODE,
};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FilePath;

/// Check if a node is `Rails.root` or `::Rails.root`.
fn is_rails_root(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"root" {
            if let Some(recv) = call.receiver() {
                return util::constant_name(&recv) == Some(b"Rails");
            }
        }
    }
    false
}

/// Check if a node is or contains Rails.root (shallow check for common patterns).
fn contains_rails_root(node: &ruby_prism::Node<'_>) -> bool {
    is_rails_root(node)
}

impl Cop for FilePath {
    fn name(&self) -> &'static str {
        "Rails/FilePath"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            EMBEDDED_STATEMENTS_NODE,
            INTERPOLATED_STRING_NODE,
            LOCAL_VARIABLE_READ_NODE,
            STRING_NODE,
        ]
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
        let style = config.get_str("EnforcedStyle", "slashes");

        // Check string interpolation: "#{Rails.root}/path/to"
        if let Some(istr) = node.as_interpolated_string_node() {
            let parts: Vec<_> = istr.parts().iter().collect();
            for (i, part) in parts.iter().enumerate() {
                if let Some(embedded) = part.as_embedded_statements_node() {
                    if let Some(stmts) = embedded.statements() {
                        let body: Vec<_> = stmts.body().iter().collect();
                        if body.len() == 1 && contains_rails_root(&body[0]) {
                            // Check if the next part starts with /
                            if i + 1 < parts.len() {
                                if let Some(str_part) = parts[i + 1].as_string_node() {
                                    if str_part.unescaped().starts_with(b"/") {
                                        let loc = node.location();
                                        let (line, column) =
                                            source.offset_to_line_col(loc.start_offset());
                                        let msg = if style == "arguments" {
                                            "Prefer `Rails.root.join('path', 'to').to_s`."
                                        } else {
                                            "Prefer `Rails.root.join('path/to').to_s`."
                                        };
                                        diagnostics.push(self.diagnostic(
                                            source,
                                            line,
                                            column,
                                            msg.to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"join" {
            return;
        }

        // Pattern 1: File.join(Rails.root, ...) — receiver is File constant
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if util::constant_name(&recv) == Some(b"File") {
            // File.join(Rails.root, ...) pattern
            let args = match call.arguments() {
                Some(a) => a,
                None => return,
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();

            // Check if any argument contains Rails.root
            let has_rails_root = arg_list.iter().any(|a| contains_rails_root(a));
            if !has_rails_root {
                return;
            }

            // Check that no arguments are plain variables or constants
            // (Rails.root itself is a send node, so it's fine)
            let has_invalid_arg = arg_list.iter().any(|a| {
                a.as_local_variable_read_node().is_some()
                    || ((a.as_constant_read_node().is_some()
                        || a.as_constant_path_node().is_some())
                        && util::constant_name(a) != Some(b"Rails"))
            });
            if has_invalid_arg {
                return;
            }

            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let msg = if style == "arguments" {
                "Prefer `Rails.root.join('path', 'to').to_s`."
            } else {
                "Prefer `Rails.root.join('path/to').to_s`."
            };
            diagnostics.push(self.diagnostic(source, line, column, msg.to_string()));
        }

        // Pattern 2: Rails.root.join('path', 'to') — receiver is Rails.root
        let root_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if root_call.name().as_slice() != b"root" {
            return;
        }
        let rails_recv = match root_call.receiver() {
            Some(r) => r,
            None => return,
        };
        if util::constant_name(&rails_recv) != Some(b"Rails") {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();

        // All args should be strings
        let all_strings = arg_list.iter().all(|a| a.as_string_node().is_some());
        if !all_strings {
            return;
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
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Prefer `Rails.root.join('path', 'to').to_s`.".to_string(),
                            ));
                        }
                    }
                }
            }
            _ => {
                // "slashes" (default): flag multi-arg join calls
                if arg_list.len() < 2 {
                    return;
                }
                // Skip if any arg contains multiple slashes
                let has_multi_slash = arg_list.iter().any(|a| {
                    if let Some(s) = a.as_string_node() {
                        let val = s.unescaped();
                        val.windows(2).any(|w| w == b"//")
                    } else {
                        false
                    }
                });
                if has_multi_slash {
                    return;
                }
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `Rails.root.join('path/to')`.".to_string(),
                ));
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
        assert!(
            !diags.is_empty(),
            "arguments style should flag slash-separated path"
        );
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
