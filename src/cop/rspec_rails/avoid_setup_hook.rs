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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be a bare `setup` call (no receiver)
        if call.name().as_slice() != b"setup" || call.receiver().is_some() {
            return Vec::new();
        }

        // Must have a block
        if call.block().is_none() {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `before` instead of `setup`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AvoidSetupHook, "cops/rspecrails/avoid_setup_hook");
}
