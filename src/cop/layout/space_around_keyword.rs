use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceAroundKeyword;

const KEYWORDS: &[&[u8]] = &[
    b"if", b"unless", b"while", b"until", b"case", b"when", b"elsif",
    b"return", b"yield",
];

impl Cop for SpaceAroundKeyword {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundKeyword"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let bytes = source.as_bytes();
        let len = bytes.len();
        let mut diagnostics = Vec::new();

        for &kw in KEYWORDS {
            let kw_len = kw.len();
            let mut i = 0;
            while i + kw_len < len {
                if &bytes[i..i + kw_len] == kw && code_map.is_code(i) {
                    // Verify it's a word boundary before
                    let word_before = if i > 0 {
                        bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'
                    } else {
                        false
                    };
                    // Check if followed by (
                    let followed_by_paren = i + kw_len < len && bytes[i + kw_len] == b'(';

                    if !word_before && followed_by_paren {
                        // Also check it's at the start of an expression (not inside a method name)
                        let at_line_start = i == 0 || bytes[i - 1] == b'\n' || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t' || bytes[i - 1] == b';';
                        if at_line_start {
                            let kw_str = std::str::from_utf8(kw).unwrap_or("");
                            let (line, column) = source.offset_to_line_col(i);
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Space missing after keyword `{kw_str}`."),
                            ));
                        }
                    }
                }
                i += 1;
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAroundKeyword, "cops/layout/space_around_keyword");
}
