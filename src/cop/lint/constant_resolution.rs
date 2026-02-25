use crate::cop::node_type::{CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that certain constants are fully qualified.
/// Disabled by default; useful for gems to avoid conflicts.
pub struct ConstantResolution;

impl Cop for ConstantResolution {
    fn name(&self) -> &'static str {
        "Lint/ConstantResolution"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        // Check for unqualified constant (no parent scope, just `Foo` not `::Foo`)
        // ConstantPathNode (qualified like Foo::Bar or ::Foo) is already resolved,
        // so we only flag simple ConstantReadNode references.
        if node.as_constant_path_node().is_some() {
            return;
        }

        let const_node = match node.as_constant_read_node() {
            Some(n) => n,
            None => return,
        };

        let name = std::str::from_utf8(const_node.name().as_slice()).unwrap_or("");

        // Check Only/Ignore config.
        // RuboCop uses `cop_config['Only'].blank?` which returns true for both
        // nil and []. So `Only: []` (the default) means "check everything", same
        // as not configuring Only at all. Only a non-empty list restricts checking.
        let only = config.get_string_array("Only").unwrap_or_default();
        let ignore = config.get_string_array("Ignore").unwrap_or_default();

        if !only.is_empty() && !only.contains(&name.to_string()) {
            return;
        }
        if ignore.contains(&name.to_string()) {
            return;
        }

        let loc = const_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Fully qualify this constant to avoid possibly ambiguous resolution.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_with_config, run_cop_full_with_config};
    use std::collections::HashMap;
    crate::cop_fixture_tests!(ConstantResolution, "cops/lint/constant_resolution");

    fn config_with_only(values: Vec<&str>) -> crate::cop::CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "Only".to_string(),
            serde_yml::Value::Sequence(
                values
                    .into_iter()
                    .map(|s| serde_yml::Value::String(s.to_string()))
                    .collect(),
            ),
        );
        crate::cop::CopConfig {
            options,
            ..crate::cop::CopConfig::default()
        }
    }

    #[test]
    fn empty_only_flags_all_constants() {
        // RuboCop's `Only: []` (the default) uses `.blank?` which returns true
        // for empty arrays, so it flags ALL unqualified constants.
        let config = config_with_only(vec![]);
        let diags = run_cop_full_with_config(&ConstantResolution, b"Foo\nBar\n", config);
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn only_restricts_to_listed_constants() {
        let config = config_with_only(vec!["Foo"]);
        let diags = run_cop_full_with_config(&ConstantResolution, b"Foo\nBar\n", config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Fully qualify"));
    }

    #[test]
    fn only_with_no_match_produces_no_offenses() {
        let config = config_with_only(vec!["Baz"]);
        assert_cop_no_offenses_with_config(&ConstantResolution, b"Foo\nBar\n", config);
    }

    #[test]
    fn ignore_suppresses_listed_constants() {
        let mut options = HashMap::new();
        options.insert(
            "Ignore".to_string(),
            serde_yml::Value::Sequence(vec![serde_yml::Value::String("Foo".to_string())]),
        );
        let config = crate::cop::CopConfig {
            options,
            ..crate::cop::CopConfig::default()
        };
        let diags = run_cop_full_with_config(&ConstantResolution, b"Foo\nBar\n", config);
        assert_eq!(diags.len(), 1);
    }
}
