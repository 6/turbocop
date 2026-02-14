use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceAfterColon;

impl Cop for SpaceAfterColon {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterColon"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let assoc = match node.as_assoc_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let key = assoc.key();
        let sym = match key.as_symbol_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let colon_loc = match sym.closing_loc() {
            Some(loc) if loc.as_slice() == b":" => loc,
            _ => return Vec::new(),
        };

        let bytes = source.as_bytes();
        let after_colon = colon_loc.end_offset();
        if bytes.get(after_colon) != Some(&b' ') {
            let (line, column) = source.offset_to_line_col(colon_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Space missing after colon.".to_string(),
            )];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAfterColon, "cops/layout/space_after_colon");
}
