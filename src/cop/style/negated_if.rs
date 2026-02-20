use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, IF_NODE};

pub struct NegatedIf;

impl Cop for NegatedIf {
    fn name(&self) -> &'static str {
        "Style/NegatedIf"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, IF_NODE]
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
        let enforced_style = config.get_str("EnforcedStyle", "both");
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Must have an `if` keyword (not ternary)
        let if_kw_loc = match if_node.if_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Must actually be `if`, not `unless`
        if if_kw_loc.as_slice() != b"if" {
            return;
        }

        // Must not have an else clause
        if if_node.subsequent().is_some() {
            return;
        }

        // Detect modifier (postfix) form: `do_something if condition`
        // In modifier form, the `if` keyword comes after the body in source
        let is_modifier = if_node.end_keyword_loc().is_none();

        // EnforcedStyle filtering
        match enforced_style {
            "prefix" if is_modifier => return,
            "postfix" if !is_modifier => return,
            _ => {} // "both" checks all forms
        }

        // Check if predicate is a `!` call
        let predicate = if_node.predicate();
        if let Some(call) = predicate.as_call_node() {
            if call.name().as_slice() == b"!" {
                let (line, column) = source.offset_to_line_col(if_kw_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, "Favor `unless` over `if` for negative conditions.".to_string()));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(NegatedIf, "cops/style/negated_if");

    #[test]
    fn enforced_style_prefix_ignores_postfix() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("prefix".into())),
            ]),
            ..CopConfig::default()
        };
        // Postfix (modifier) form should be ignored with "prefix" style
        let source = b"do_something if !condition\n";
        let diags = run_cop_full_with_config(&NegatedIf, source, config);
        assert!(diags.is_empty(), "Should ignore modifier form with prefix style");
    }

    #[test]
    fn enforced_style_prefix_flags_prefix() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("prefix".into())),
            ]),
            ..CopConfig::default()
        };
        // Prefix form should still be flagged
        let source = b"if !condition\n  do_something\nend\n";
        let diags = run_cop_full_with_config(&NegatedIf, source, config);
        assert_eq!(diags.len(), 1, "Should flag prefix form with prefix style");
    }

    #[test]
    fn enforced_style_postfix_ignores_prefix() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("postfix".into())),
            ]),
            ..CopConfig::default()
        };
        // Prefix form should be ignored with "postfix" style
        let source = b"if !condition\n  do_something\nend\n";
        let diags = run_cop_full_with_config(&NegatedIf, source, config);
        assert!(diags.is_empty(), "Should ignore prefix form with postfix style");
    }

    #[test]
    fn enforced_style_postfix_flags_postfix() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("postfix".into())),
            ]),
            ..CopConfig::default()
        };
        let source = b"do_something if !condition\n";
        let diags = run_cop_full_with_config(&NegatedIf, source, config);
        assert_eq!(diags.len(), 1, "Should flag modifier form with postfix style");
    }
}
