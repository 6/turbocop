use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceBeforeSemicolon;

impl Cop for SpaceBeforeSemicolon {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeSemicolon"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let bytes = source.as_bytes();
        let mut diagnostics = Vec::new();
        for (i, &byte) in bytes.iter().enumerate() {
            if byte == b';' && i > 0 && bytes[i - 1] == b' ' && code_map.is_code(i) {
                let (line, column) = source.offset_to_line_col(i - 1);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Space found before semicolon.".to_string(),
                ));
            }
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeSemicolon, "cops/layout/space_before_semicolon");
}
