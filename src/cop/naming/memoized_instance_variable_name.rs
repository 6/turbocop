use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{DEF_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE, STATEMENTS_NODE};

pub struct MemoizedInstanceVariableName;

impl MemoizedInstanceVariableName {
    fn check_or_write(
        &self,
        source: &SourceFile,
        or_write: ruby_prism::InstanceVariableOrWriteNode<'_>,
        base_name: &str,
        method_name_str: &str,
        leading_underscore_style: &str,
    ) -> Vec<Diagnostic> {
        let ivar_name = or_write.name().as_slice();
        let ivar_str = std::str::from_utf8(ivar_name).unwrap_or("");
        let ivar_base = ivar_str.strip_prefix('@').unwrap_or(ivar_str);

        let matches = match leading_underscore_style {
            "required" => {
                // @_method_name is the only valid form
                let expected = format!("_{base_name}");
                ivar_base == expected
            }
            "optional" => {
                // Both @method_name and @_method_name are valid
                let with_underscore = format!("_{base_name}");
                ivar_base == base_name || ivar_base == with_underscore
            }
            _ => {
                // "disallowed" (default): only @method_name is valid
                ivar_base == base_name
            }
        };

        if !matches {
            let loc = or_write.name_loc();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Memoized variable `@{ivar_base}` does not match method name `{method_name_str}`."
                ),
            )];
        }

        Vec::new()
    }
}

impl Cop for MemoizedInstanceVariableName {
    fn name(&self) -> &'static str {
        "Naming/MemoizedInstanceVariableName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let enforced_style = config.get_str("EnforcedStyleForLeadingUnderscores", "disallowed");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let method_name = def_node.name().as_slice();
        let method_name_str = std::str::from_utf8(method_name).unwrap_or("");

        // RuboCop skips initialize methods — `||=` there is default initialization, not memoization
        if matches!(
            method_name_str,
            "initialize" | "initialize_clone" | "initialize_copy" | "initialize_dup"
        ) {
            return;
        }

        // Strip trailing ? or ! from method name for matching
        let base_name = method_name_str.trim_end_matches(|c| c == '?' || c == '!');

        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        // Look for @var ||= pattern — only when it's the entire body or the last statement.
        // This is a memoization pattern; a `||=` in the middle of a method is just assignment.

        // Body could be a bare InstanceVariableOrWriteNode (single statement)
        if let Some(or_write) = body.as_instance_variable_or_write_node() {
            diagnostics.extend(self.check_or_write(source, or_write, base_name, method_name_str, enforced_style));
        }

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.is_empty() {
            return;
        }

        // Only check the last statement — vendor requires ||= be the sole or last statement
        let last = &body_nodes[body_nodes.len() - 1];
        if let Some(or_write) = last.as_instance_variable_or_write_node() {
            diagnostics.extend(self.check_or_write(source, or_write, base_name, method_name_str, enforced_style));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MemoizedInstanceVariableName,
        "cops/naming/memoized_instance_variable_name"
    );

    #[test]
    fn required_style_allows_leading_underscore() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleForLeadingUnderscores".to_string(),
                serde_yml::Value::String("required".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def js_modules\n  @_js_modules ||= compute_modules\nend\n";
        assert_cop_no_offenses_full_with_config(&MemoizedInstanceVariableName, source, config);
    }

    #[test]
    fn optional_style_allows_both_forms() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleForLeadingUnderscores".to_string(),
                serde_yml::Value::String("optional".to_string()),
            )]),
            ..CopConfig::default()
        };
        // Both forms should be accepted
        let source = b"def js_modules\n  @_js_modules ||= compute_modules\nend\n";
        assert_cop_no_offenses_full_with_config(
            &MemoizedInstanceVariableName,
            source,
            config.clone(),
        );
        let source2 = b"def js_modules\n  @js_modules ||= compute_modules\nend\n";
        assert_cop_no_offenses_full_with_config(&MemoizedInstanceVariableName, source2, config);
    }

    #[test]
    fn required_style_flags_missing_underscore() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyleForLeadingUnderscores".to_string(),
                serde_yml::Value::String("required".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def js_modules\n  @js_modules ||= compute_modules\nend\n";
        let diags = run_cop_full_with_config(&MemoizedInstanceVariableName, source, config);
        assert!(
            !diags.is_empty(),
            "required style should flag @js_modules (missing underscore)"
        );
    }
}
