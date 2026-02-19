use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE};

pub struct StartWith;

/// Check if regex content starts with \A and the rest is a simple literal.
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

impl Cop for StartWith {
    fn name(&self) -> &'static str {
        "Performance/StartWith"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, REGULAR_EXPRESSION_NODE]
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

        if call.name().as_slice() != b"match?" {
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
        let first_arg = match args.iter().next() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let regex_node = match first_arg.as_regular_expression_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let content = regex_node.content_loc().as_slice();
        if !is_start_anchored_literal(content, safe_multiline) {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `start_with?` instead of a regex match anchored to the beginning of the string.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(StartWith, "cops/performance/start_with");

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
        let source = b"'abc'.match?(/^ab/)\n";
        let diags = run_cop_full_with_config(&StartWith, source, config);
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
        let source = b"'abc'.match?(/^ab/)\n";
        let diags = run_cop_full_with_config(&StartWith, source, config);
        assert!(diags.is_empty(), "Should not flag ^anchor when SafeMultiline:true");
    }
}
