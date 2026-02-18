use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DoubleCopDisableDirective;

impl Cop for DoubleCopDisableDirective {
    fn name(&self) -> &'static str {
        "Style/DoubleCopDisableDirective"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Compute line byte offsets for heredoc checking
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut line_offsets = Vec::with_capacity(lines.len());
        let mut offset = 0usize;
        for line in &lines {
            line_offsets.push(offset);
            offset += line.len() + 1;
        }

        for (i, line) in lines.iter().enumerate() {
            // Skip lines inside heredocs
            if i < line_offsets.len() && code_map.is_heredoc(line_offsets[i]) {
                continue;
            }

            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Find first rubocop:disable or rubocop:todo directive
            let first_pos = line_str
                .find("# rubocop:disable ")
                .or_else(|| line_str.find("# rubocop:todo "));

            let first_pos = match first_pos {
                Some(p) => p,
                None => continue,
            };

            // Check if there's a second directive on the same line.
            // Skip past the entire first directive prefix to avoid self-matching.
            let skip_len = if line_str[first_pos..].starts_with("# rubocop:disable ") {
                "# rubocop:disable ".len()
            } else {
                "# rubocop:todo ".len()
            };
            let after_first = first_pos + skip_len;
            let rest = &line_str[after_first..];
            if rest.contains("# rubocop:disable ") || rest.contains("# rubocop:todo ") {
                let col = first_pos;
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    col,
                    "More than one disable comment on one line.".to_string(),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DoubleCopDisableDirective, "cops/style/double_cop_disable_directive");
}
