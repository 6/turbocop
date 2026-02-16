use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ForCop;

impl Cop for ForCop {
    fn name(&self) -> &'static str {
        "Style/For"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "each");

        if enforced_style != "each" {
            return Vec::new();
        }

        let for_node = match node.as_for_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let loc = for_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer `each` over `for`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ForCop, "cops/style/for_cop");
}
