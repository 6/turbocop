use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SafeNavigation;

impl Cop for SafeNavigation {
    fn name(&self) -> &'static str {
        "Rails/SafeNavigation"
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
        let convert_try = config.get_bool("ConvertTry", false);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();

        // Always flag try!
        // Only flag try when ConvertTry is true
        if name == b"try" && !convert_try {
            return Vec::new();
        }
        if name != b"try" && name != b"try!" {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use safe navigation (`&.`) instead of `try`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SafeNavigation, "cops/rails/safe_navigation");

    #[test]
    fn convert_try_false_skips_try() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;

        let config = CopConfig::default();
        let source = b"foo.try(:bar)\n";
        assert_cop_no_offenses_full_with_config(&SafeNavigation, source, config);
    }

    #[test]
    fn convert_try_true_flags_try() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "ConvertTry".to_string(),
                serde_yml::Value::Bool(true),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo.try(:bar)\n";
        let diags = run_cop_full_with_config(&SafeNavigation, source, config);
        assert!(!diags.is_empty(), "ConvertTry:true should flag try");
    }

    #[test]
    fn convert_try_false_still_flags_try_bang() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig::default();
        let source = b"foo.try!(:bar)\n";
        let diags = run_cop_full_with_config(&SafeNavigation, source, config);
        assert!(!diags.is_empty(), "ConvertTry:false should still flag try!");
    }
}
