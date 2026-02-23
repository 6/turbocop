use crate::cop::node_type::CALL_NODE;
use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AvoidSetupHook;

impl Cop for AvoidSetupHook {
    fn name(&self) -> &'static str {
        "RSpecRails/AvoidSetupHook"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_RAILS_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be a bare `setup` call (no receiver)
        if call.name().as_slice() != b"setup" || call.receiver().is_some() {
            return;
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        // Skip if the block has parameters — DSL methods like Karafka's
        // `setup do |config| ... end` take a block parameter, whereas
        // the minitest/Rails `setup` hook does not.
        if let Some(block_node) = block.as_block_node() {
            if block_node.parameters().is_some() {
                return;
            }
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `before` instead of `setup`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AvoidSetupHook, "cops/rspecrails/avoid_setup_hook");
}
