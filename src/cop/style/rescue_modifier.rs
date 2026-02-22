use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::RESCUE_MODIFIER_NODE;

pub struct RescueModifier;

impl Cop for RescueModifier {
    fn name(&self) -> &'static str {
        "Style/RescueModifier"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[RESCUE_MODIFIER_NODE]
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
        let rescue_mod = match node.as_rescue_modifier_node() {
            Some(r) => r,
            None => return,
        };

        // RuboCop points at the whole rescue modifier expression, not just the `rescue` keyword
        let loc = rescue_mod.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, "Avoid rescuing without specifying an error class.".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(RescueModifier, "cops/style/rescue_modifier");

    #[test]
    fn inline_rescue_fires() {
        let source = b"x = foo rescue nil\n";
        let diags = run_cop_full(&RescueModifier, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Avoid rescuing"));
    }
}
