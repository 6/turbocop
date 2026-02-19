use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE};

pub struct DeleteSuffix;

fn is_end_anchored_literal(content: &[u8], safe_multiline: bool) -> bool {
    if content.len() < 2 {
        return false;
    }
    // Check for \z anchor (always valid)
    if content.len() >= 3 && content[content.len() - 2] == b'\\' && content[content.len() - 1] == b'z' {
        let prefix = &content[..content.len() - 2];
        if !prefix.is_empty() && is_literal_chars(prefix) {
            return true;
        }
    }
    // Check for $ anchor (only when SafeMultiline is false)
    if !safe_multiline && content[content.len() - 1] == b'$' {
        let prefix = &content[..content.len() - 1];
        if !prefix.is_empty() && is_literal_chars(prefix) {
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

impl Cop for DeleteSuffix {
    fn name(&self) -> &'static str {
        "Performance/DeleteSuffix"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE]
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

        // First arg must be a regex ending with \z and literal prefix
        let regex_node = match first_arg.as_regular_expression_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Skip if regex has flags (e.g., /pattern\z/i) — delete_suffix can't replicate flags
        let closing = regex_node.closing_loc().as_slice();
        if closing.len() > 1 {
            return Vec::new();
        }

        let content = regex_node.content_loc().as_slice();
        if !is_end_anchored_literal(content, safe_multiline) {
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
        vec![self.diagnostic(source, line, column, "Use `delete_suffix` instead of `gsub`.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DeleteSuffix, "cops/performance/delete_suffix");

    #[test]
    fn config_safe_multiline_false_flags_dollar() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("SafeMultiline".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let source = b"str.gsub(/suffix$/, '')\n";
        let diags = run_cop_full_with_config(&DeleteSuffix, source, config);
        assert!(!diags.is_empty(), "Should flag $anchor when SafeMultiline:false");
    }

    #[test]
    fn config_safe_multiline_true_ignores_dollar() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("SafeMultiline".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let source = b"str.gsub(/suffix$/, '')\n";
        let diags = run_cop_full_with_config(&DeleteSuffix, source, config);
        assert!(diags.is_empty(), "Should not flag $anchor when SafeMultiline:true");
    }
}
