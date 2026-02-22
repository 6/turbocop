use crate::cop::node_type::FOR_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ForCop;

impl Cop for ForCop {
    fn name(&self) -> &'static str {
        "Style/For"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[FOR_NODE]
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
        let enforced_style = config.get_str("EnforcedStyle", "each");

        if enforced_style != "each" {
            return;
        }

        let for_node = match node.as_for_node() {
            Some(n) => n,
            None => return,
        };

        let loc = for_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Prefer `each` over `for`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ForCop, "cops/style/for_cop");
}
