use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceAroundKeyword;

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
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let bytes = source.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        // Single-pass scan: dispatch on first byte to candidate keywords.
        // Keywords grouped by first letter: c(ase), e(lsif), i(f), r(eturn),
        // u(nless/ntil), w(hile/hen).
        while i < len {
            let candidates: &[&[u8]] = match bytes[i] {
                b'c' => &[b"case"],
                b'e' => &[b"elsif"],
                b'i' => &[b"if"],
                b'r' => &[b"return"],
                b'u' => &[b"unless", b"until"],
                b'w' => &[b"while", b"when"],
                _ => {
                    i += 1;
                    continue;
                }
            };

            for &kw in candidates {
                let kw_len = kw.len();
                if i + kw_len < len
                    && &bytes[i..i + kw_len] == kw
                    && code_map.is_code(i)
                {
                    let word_before = if i > 0 {
                        bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'
                    } else {
                        false
                    };
                    let followed_by_paren = bytes[i + kw_len] == b'(';

                    if !word_before && followed_by_paren {
                        let at_line_start = i == 0
                            || bytes[i - 1] == b'\n'
                            || bytes[i - 1] == b' '
                            || bytes[i - 1] == b'\t'
                            || bytes[i - 1] == b';';
                        let preceded_by_def = i >= 4 && &bytes[i - 4..i] == b"def ";
                        if at_line_start && !preceded_by_def {
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
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAroundKeyword, "cops/layout/space_around_keyword");
}
