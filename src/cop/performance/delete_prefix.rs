use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DeletePrefix;

fn is_start_anchored_literal(content: &[u8], safe_multiline: bool) -> bool {
    if content.len() < 2 {
        return false;
    }
    // Check for \A anchor (always valid)
    if content.len() >= 3 && content[0] == b'\\' && content[1] == b'A' {
        let rest = &content[2..];
        if !rest.is_empty() && is_literal_chars(rest) {
            return true;
        }
    }
    // Check for ^ anchor (only when SafeMultiline is false)
    if !safe_multiline && content[0] == b'^' {
        let rest = &content[1..];
        if !rest.is_empty() && is_literal_chars(rest) {
            return true;
        }
    }
    false
}

fn is_literal_chars(bytes: &[u8]) -> bool {
    for &b in bytes {
        match b {
            b'.' | b'*' | b'+' | b'?' | b'|' | b'(' | b')' | b'[' | b']' | b'{' | b'}'
            | b'^' | b'$' | b'\\' => return false,
            _ => {}
        }
    }
    true
}

impl Cop for DeletePrefix {
    fn name(&self) -> &'static str {
        "Performance/DeletePrefix"
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
        let safe_multiline = config.get_bool("SafeMultiline", true);
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"gsub" && method_name != b"sub" {
            return Vec::new();
        }

        if call.receiver().is_none() {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        if args.len() != 2 {
            return Vec::new();
        }

        let mut iter = args.iter();
        let first_arg = iter.next().unwrap();
        let second_arg = iter.next().unwrap();

        // First arg must be a regex starting with \A and literal rest
        let regex_node = match first_arg.as_regular_expression_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let content = regex_node.content_loc().as_slice();
        if !is_start_anchored_literal(content, safe_multiline) {
            return Vec::new();
        }

        // Second arg must be an empty string
        let string_node = match second_arg.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        if !string_node.unescaped().is_empty() {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `delete_prefix` instead of `gsub`.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DeletePrefix, "cops/performance/delete_prefix");

    #[test]
    fn config_safe_multiline_false_flags_caret() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("SafeMultiline".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let source = b"str.gsub(/^prefix/, '')\n";
        let diags = run_cop_full_with_config(&DeletePrefix, source, config);
        assert!(!diags.is_empty(), "Should flag ^anchor when SafeMultiline:false");
    }

    #[test]
    fn config_safe_multiline_true_ignores_caret() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("SafeMultiline".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let source = b"str.gsub(/^prefix/, '')\n";
        let diags = run_cop_full_with_config(&DeletePrefix, source, config);
        assert!(diags.is_empty(), "Should not flag ^anchor when SafeMultiline:true");
    }
}
