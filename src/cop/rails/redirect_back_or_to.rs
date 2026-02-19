use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct RedirectBackOrTo;

impl Cop for RedirectBackOrTo {
    fn name(&self) -> &'static str {
        "Rails/RedirectBackOrTo"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // minimum_target_rails_version 7.0
        // redirect_back_or_to was introduced in Rails 7.0; skip for older versions.
        let rails_version = config
            .options
            .get("TargetRailsVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(5.0);
        if rails_version < 7.0 {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be receiverless `redirect_back`
        if call.receiver().is_some() || call.name().as_slice() != b"redirect_back" {
            return;
        }

        // Must have `fallback_location:` keyword argument
        if keyword_arg_value(&call, b"fallback_location").is_none() {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `redirect_back_or_to` instead of `redirect_back` with `:fallback_location` keyword argument.".to_string(),
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
            &RedirectBackOrTo,
            include_bytes!("../../../testdata/cops/rails/redirect_back_or_to/offense.rb"),
            config_with_rails(7.0),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RedirectBackOrTo,
            include_bytes!("../../../testdata/cops/rails/redirect_back_or_to/no_offense.rb"),
            config_with_rails(7.0),
        );
    }

    #[test]
    fn skipped_when_rails_below_7() {
        let source = b"redirect_back(fallback_location: root_path)\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &RedirectBackOrTo,
            source,
            config_with_rails(6.1),
            "test.rb",
        );
        assert!(diagnostics.is_empty(), "Should not fire on Rails < 7.0");
    }

    #[test]
    fn skipped_when_no_rails_version() {
        let source = b"redirect_back(fallback_location: root_path)\n";
        let diagnostics = crate::testutil::run_cop_full_internal(
            &RedirectBackOrTo,
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
