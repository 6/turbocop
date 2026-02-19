use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct IndentationStyle;

impl Cop for IndentationStyle {
    fn name(&self) -> &'static str {
        "Layout/IndentationStyle"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let style = config.get_str("EnforcedStyle", "spaces");
        let _indent_width = config.get_usize("IndentationWidth", 2);

        let mut offset = 0;

        for (i, line) in source.lines().enumerate() {
            let line_num = i + 1;
            let line_start = offset;
            // Advance offset past this line and its newline
            offset += line.len() + 1; // +1 for the '\n' delimiter

            // Skip lines whose indentation starts in a non-code region (heredocs, strings)
            if !code_map.is_code(line_start) {
                continue;
            }

            if style == "spaces" {
                // Flag tabs in indentation
                let indent_end = line.iter().take_while(|&&b| b == b' ' || b == b'\t').count();
                let indent = &line[..indent_end];
                if indent.iter().any(|&b| b == b'\t') {
                    let tab_col = indent.iter().position(|&b| b == b'\t').unwrap_or(0);
                    let tab_offset = line_start + tab_col;
                    // Double-check the specific tab character is in a code region
                    if code_map.is_code(tab_offset) {
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            tab_col,
                            "Tab detected in indentation.".to_string(),
                        ));
                    }
                }
            } else {
                // "tabs" — flag spaces in indentation
                let indent_end = line.iter().take_while(|&&b| b == b' ' || b == b'\t').count();
                let indent = &line[..indent_end];
                if indent.iter().any(|&b| b == b' ') {
                    let space_col = indent.iter().position(|&b| b == b' ').unwrap_or(0);
                    let space_offset = line_start + space_col;
                    if code_map.is_code(space_offset) {
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            space_col,
                            "Space detected in indentation.".to_string(),
                        ));
                    }
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(IndentationStyle, "cops/layout/indentation_style");
}
