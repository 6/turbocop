use crate::cop::{Cop, CopConfig, EnabledState};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct PluckId;

impl Cop for PluckId {
    fn name(&self) -> &'static str {
        "Rails/PluckId"
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
        // This cop is disabled by default in RuboCop (Enabled: false in vendor config).
        // Only fire when explicitly enabled in the project config.
        if config.enabled != EnabledState::True {
            return Vec::new();
        }
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"pluck" {
            return Vec::new();
        }

        // Must have a receiver (chained call)
        if call.receiver().is_none() {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        if let Some(sym) = arg_list[0].as_symbol_node() {
            if sym.unescaped() == b"id" {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `ids` instead of `pluck(:id)`.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_config() -> CopConfig {
        CopConfig {
            enabled: EnabledState::True,
            ..CopConfig::default()
        }
    }

    #[test]
    fn offense_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &PluckId,
            include_bytes!("../../../testdata/cops/rails/pluck_id/offense.rb"),
            enabled_config(),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &PluckId,
            include_bytes!("../../../testdata/cops/rails/pluck_id/no_offense.rb"),
            enabled_config(),
        );
    }
}
