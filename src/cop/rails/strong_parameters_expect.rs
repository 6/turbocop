use crate::cop::node_type::{CALL_NODE, LOCAL_VARIABLE_READ_NODE};
use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct StrongParametersExpect;

/// Check if a node is a `params` receiver (local variable or method call).
fn is_params_receiver(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        return call.name().as_slice() == b"params" && call.receiver().is_none();
    }
    if let Some(lvar) = node.as_local_variable_read_node() {
        return lvar.name().as_slice() == b"params";
    }
    false
}

impl Cop for StrongParametersExpect {
    fn name(&self) -> &'static str {
        "Rails/StrongParametersExpect"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/controllers/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, LOCAL_VARIABLE_READ_NODE]
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
        // minimum_target_rails_version 8.0
        // params.expect(...) was introduced in Rails 8.0; skip for older versions.
        let rails_version = config
            .options
            .get("TargetRailsVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(5.0);
        if rails_version < 8.0 {
            return;
        }

        // Pattern 1: params.require(:x).permit(:a, :b)
        // Pattern 2: params.permit(x: [:a, :b]).require(:x)
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        let is_require_permit = chain.inner_method == b"require" && chain.outer_method == b"permit";
        let is_permit_require = chain.inner_method == b"permit" && chain.outer_method == b"require";

        if !is_require_permit && !is_permit_require {
            return;
        }

        // Check if the innermost receiver is `params`
        let inner_receiver = match chain.inner_call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !is_params_receiver(&inner_receiver) {
            return;
        }

        // For require.permit, permit must have arguments
        if is_require_permit {
            let outer_call = node.as_call_node().unwrap();
            if outer_call.arguments().is_none() {
                return;
            }
        }

        let msg_loc = chain
            .inner_call
            .message_loc()
            .unwrap_or(chain.inner_call.location());

        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `expect(...)` instead.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use std::collections::HashMap;

    fn config_with_rails(version: f64) -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "TargetRailsVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(version)),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn offense_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &StrongParametersExpect,
            include_bytes!("../../../testdata/cops/rails/strong_parameters_expect/offense.rb"),
            config_with_rails(8.0),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &StrongParametersExpect,
            include_bytes!("../../../testdata/cops/rails/strong_parameters_expect/no_offense.rb"),
            config_with_rails(8.0),
        );
    }

    #[test]
    fn skipped_when_rails_below_8() {
        // On Rails 7.x, the cop should never fire
        let source = b"params.require(:user).permit(:name)\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &StrongParametersExpect,
            source,
            config_with_rails(7.1),
            "test.rb",
        );
        assert!(diagnostics.is_empty(), "Should not fire on Rails < 8.0");
    }

    #[test]
    fn skipped_when_no_rails_version() {
        // Default (no TargetRailsVersion) should be 5.0, so cop doesn't fire
        let source = b"params.require(:user).permit(:name)\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &StrongParametersExpect,
            source,
            CopConfig::default(),
            "test.rb",
        );
        assert!(
            diagnostics.is_empty(),
            "Should not fire when TargetRailsVersion defaults to 5.0"
        );
    }
}
