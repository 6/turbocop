use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, IMPLICIT_NODE, SYMBOL_NODE};

pub struct SpaceAfterColon;

impl Cop for SpaceAfterColon {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterColon"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, IMPLICIT_NODE, SYMBOL_NODE]
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
        let assoc = match node.as_assoc_node() {
            Some(a) => a,
            None => return,
        };

        // Skip value-omission shorthand hash syntax (Ruby 3.1+): { url:, driver: }
        // In Prism, when value is omitted, the value node is an ImplicitNode.
        if assoc.value().as_implicit_node().is_some() {
            return;
        }

        let key = assoc.key();
        let sym = match key.as_symbol_node() {
            Some(s) => s,
            None => return,
        };

        let colon_loc = match sym.closing_loc() {
            Some(loc) if loc.as_slice() == b":" => loc,
            _ => return,
        };

        let bytes = source.as_bytes();
        let after_colon = colon_loc.end_offset();
        // RuboCop accepts any whitespace after colon (space, newline, tab)
        match bytes.get(after_colon) {
            Some(b) if b.is_ascii_whitespace() => {}
            _ => {
                let (line, column) = source.offset_to_line_col(colon_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Space missing after colon.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAfterColon, "cops/layout/space_after_colon");
}
