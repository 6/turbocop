use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceAroundOperators;

impl Cop for SpaceAroundOperators {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundOperators"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _allow_for_alignment = config.get_bool("AllowForAlignment", true);
        let _enforced_style_exponent = config.get_str("EnforcedStyleForExponentOperator", "no_space");
        let _enforced_style_rational = config.get_str("EnforcedStyleForRationalLiterals", "no_space");
        let bytes = source.as_bytes();
        let len = bytes.len();
        let mut diagnostics = Vec::new();
        let mut i = 0;

        while i < len {
            if !code_map.is_code(i) {
                i += 1;
                continue;
            }

            // Check for multi-char operators first: ==, !=, =>
            if i + 1 < len && code_map.is_code(i + 1) {
                let two = &bytes[i..i + 2];
                if two == b"==" || two == b"!=" || two == b"=>" {
                    // Skip ===
                    if two == b"==" && i + 2 < len && bytes[i + 2] == b'=' {
                        i += 3;
                        continue;
                    }
                    let op_str = std::str::from_utf8(two).unwrap_or("??");
                    let space_before = i > 0 && bytes[i - 1] == b' ';
                    let space_after = i + 2 < len && bytes[i + 2] == b' ';
                    let newline_after = i + 2 >= len || bytes[i + 2] == b'\n' || bytes[i + 2] == b'\r';
                    if !space_before || (!space_after && !newline_after) {
                        let (line, column) = source.offset_to_line_col(i);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Surrounding space missing for operator `{op_str}`."),
                        ));
                    }
                    i += 2;
                    continue;
                }
            }

            // Single = (not ==, !=, =>, =~, <=, >=, or part of +=/-=/etc.)
            if bytes[i] == b'=' {
                // Skip =~ and =>
                if i + 1 < len && (bytes[i + 1] == b'~' || bytes[i + 1] == b'>') {
                    i += 2;
                    continue;
                }
                // Skip ==
                if i + 1 < len && bytes[i + 1] == b'=' {
                    i += 2;
                    continue;
                }
                // Skip if preceded by !, <, >, =, +, -, *, /, %, &, |, ^, ~
                if i > 0 {
                    let prev = bytes[i - 1];
                    if matches!(prev, b'!' | b'<' | b'>' | b'=' | b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' | b'~') {
                        i += 1;
                        continue;
                    }
                }

                let space_before = i > 0 && bytes[i - 1] == b' ';
                let space_after = i + 1 < len && bytes[i + 1] == b' ';
                let newline_after = i + 1 >= len || bytes[i + 1] == b'\n' || bytes[i + 1] == b'\r';
                if !space_before || (!space_after && !newline_after) {
                    let (line, column) = source.offset_to_line_col(i);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Surrounding space missing for operator `=`.".to_string(),
                    ));
                }
                i += 1;
                continue;
            }

            i += 1;
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAroundOperators, "cops/layout/space_around_operators");
}
