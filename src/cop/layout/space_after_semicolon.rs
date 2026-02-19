use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceAfterSemicolon;

impl Cop for SpaceAfterSemicolon {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterSemicolon"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let bytes = source.as_bytes();
        for (i, &byte) in bytes.iter().enumerate() {
            if byte == b';' && code_map.is_code(i) {
                let next = bytes.get(i + 1).copied();
                if !matches!(next, Some(b' ') | Some(b'\n') | Some(b'\r') | None) {
                    let (line, column) = source.offset_to_line_col(i);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space missing after semicolon.".to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAfterSemicolon, "cops/layout/space_after_semicolon");
}
