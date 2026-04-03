use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::{self, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Enforces custom RSpec dialects via the `PreferredMethods` config.
///
/// RuboCop's matcher is `(send #rspec? #ALL.all ...)` which matches any
/// `send` node (no block required) where the receiver is nil/RSpec/::RSpec
/// and the method name is in the Language `ALL` module (union of all RSpec
/// DSL categories).
///
/// ## Bug fixes (2026-03-05)
/// 1. **Removed block requirement**: The original implementation required
///    `call.block().is_some()`, but RuboCop's `(send ...)` matcher does NOT
///    require a block. Error matchers like `raise_exception(StandardError)`
///    have args but no block, and many other RSpec DSL calls (expectations,
///    runners, includes) also lack blocks.
/// 2. **Added ALL methods check**: Added the full set of RSpec Language DSL
///    methods (from `ALL.all`) to match RuboCop's behavior. This includes
///    error matchers (`raise_error`, `raise_exception`), expectations
///    (`expect`, `is_expected`), runners (`to`, `to_not`, `not_to`),
///    includes, hooks, helpers, subjects, and all example group/example
///    variants.
pub struct Dialect;

/// All RSpec DSL method names from RuboCop's `RSpec::Language::ALL.all`.
/// This is the union of ExampleGroups, Examples, Expectations, Helpers,
/// Hooks, ErrorMatchers, Includes, SharedGroups, Subjects, and Runners.
const RSPEC_ALL_METHODS: &[&str] = &[
    // ExampleGroups - Regular
    "describe",
    "context",
    "feature",
    "example_group",
    // ExampleGroups - Skipped
    "xdescribe",
    "xcontext",
    "xfeature",
    // ExampleGroups - Focused
    "fdescribe",
    "fcontext",
    "ffeature",
    // Examples - Regular
    "it",
    "specify",
    "example",
    "scenario",
    "its",
    // Examples - Focused
    "fit",
    "fspecify",
    "fexample",
    "fscenario",
    "focus",
    // Examples - Skipped
    "xit",
    "xspecify",
    "xexample",
    "xscenario",
    "skip",
    // Examples - Pending
    "pending",
    // Expectations
    "are_expected",
    "expect",
    "expect_any_instance_of",
    "is_expected",
    "should",
    "should_not",
    "should_not_receive",
    "should_receive",
    // Helpers
    "let",
    "let!",
    // Hooks
    "prepend_before",
    "before",
    "append_before",
    "around",
    "prepend_after",
    "after",
    "append_after",
    // ErrorMatchers
    "raise_error",
    "raise_exception",
    // Includes - Examples
    "it_behaves_like",
    "it_should_behave_like",
    "include_examples",
    // Includes - Context
    "include_context",
    // SharedGroups - Examples
    "shared_examples",
    "shared_examples_for",
    // SharedGroups - Context
    "shared_context",
    // Subjects
    "subject",
    "subject!",
    // Runners
    "to",
    "to_not",
    "not_to",
];

impl Cop for Dialect {
    fn name(&self) -> &'static str {
        "RSpec/Dialect"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        let method_str = match std::str::from_utf8(method_name) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Must be a known RSpec DSL method (matches RuboCop's #ALL.all check)
        if !RSPEC_ALL_METHODS.contains(&method_str) {
            return;
        }

        // Read PreferredMethods from config. RuboCop default is empty — no aliases
        // are enforced unless explicitly configured.
        let preferred = match config.options.get("PreferredMethods") {
            Some(serde_yml::Value::Mapping(map)) => map,
            _ => return,
        };

        // Check if this method is a non-preferred alias
        let preferred_name = match preferred.get(serde_yml::Value::String(method_str.to_string())) {
            Some(v) => match v.as_str() {
                Some(s) => s.trim_start_matches(':'),
                None => return,
            },
            None => return,
        };

        // Must be receiverless or RSpec.method / ::RSpec.method
        let is_rspec_call = if call.receiver().is_none() {
            true
        } else if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
        } else {
            false
        };

        if !is_rspec_call {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{preferred_name}` over `{method_str}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use std::collections::HashMap;

    fn config_with_preferred(methods: &[(&str, &str)]) -> CopConfig {
        let mut map = serde_yml::Mapping::new();
        for &(bad, good) in methods {
            map.insert(
                serde_yml::Value::String(bad.to_string()),
                serde_yml::Value::String(format!(":{good}")),
            );
        }
        let mut options = HashMap::new();
        options.insert(
            "PreferredMethods".to_string(),
            serde_yml::Value::Mapping(map),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn offense_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Dialect,
            include_bytes!("../../../tests/fixtures/cops/rspec/dialect/offense.rb"),
            config_with_preferred(&[("context", "describe"), ("raise_exception", "raise_error")]),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &Dialect,
            include_bytes!("../../../tests/fixtures/cops/rspec/dialect/no_offense.rb"),
            config_with_preferred(&[("context", "describe"), ("raise_exception", "raise_error")]),
        );
    }

    #[test]
    fn no_preferred_methods_means_no_offenses() {
        crate::testutil::assert_cop_no_offenses_full(
            &Dialect,
            b"context 'test' do\n  it 'works' do\n    expect(true).to eq(true)\n  end\nend\n",
        );
    }
}
