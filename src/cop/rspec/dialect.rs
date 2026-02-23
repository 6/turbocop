use crate::cop::node_type::CALL_NODE;
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Dialect;

impl Cop for Dialect {
    fn name(&self) -> &'static str {
        "RSpec/Dialect"
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

        // Must have a block to be an RSpec DSL call
        if call.block().is_none() {
            return;
        }

        let method_name = call.name().as_slice();
        let method_str = match std::str::from_utf8(method_name) {
            Ok(s) => s,
            Err(_) => return,
        };

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
            config_with_preferred(&[("context", "describe")]),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &Dialect,
            include_bytes!("../../../tests/fixtures/cops/rspec/dialect/no_offense.rb"),
            config_with_preferred(&[("context", "describe")]),
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
