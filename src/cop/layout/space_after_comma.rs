use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceAfterComma;

impl Cop for SpaceAfterComma {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterComma"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();
        for (i, &byte) in bytes.iter().enumerate() {
            if byte == b',' && code_map.is_code(i) {
                // Skip if this comma is part of a global variable ($, or $;)
                if i > 0 && bytes[i - 1] == b'$' {
                    continue;
                }
                let next = bytes.get(i + 1).copied();
                // Skip commas before closing delimiters — RuboCop's
                // SpaceAfterPunctuation#allowed_type? skips ), ], and |.
                if matches!(next, Some(b')') | Some(b']') | Some(b'|')) {
                    continue;
                }
                if !matches!(next, Some(b' ') | Some(b'\n') | Some(b'\r') | None) {
                    let (line, column) = source.offset_to_line_col(i);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space missing after comma.".to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAfterComma, "cops/layout/space_after_comma");
}
