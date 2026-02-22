use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
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
        let convert_try = config.get_bool("ConvertTry", false);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();

        // Always flag try!
        // Only flag try when ConvertTry is true
        if name == b"try" && !convert_try {
            return;
        }
        if name != b"try" && name != b"try!" {
            return;
        }

        // First argument must be a symbol (method name to call).
        // e.g., foo.try(:bar) or foo.try!(:baz).
        // If it's a variable or other expression, safe navigation doesn't apply.
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let first_arg = match args.arguments().iter().next() {
            Some(a) => a,
            None => return,
        };
        if first_arg.as_symbol_node().is_none() {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use safe navigation (`&.`) instead of `{}`.",
                String::from_utf8_lossy(name),
            ),
        ));
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
            options: HashMap::from([("ConvertTry".to_string(), serde_yml::Value::Bool(true))]),
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
