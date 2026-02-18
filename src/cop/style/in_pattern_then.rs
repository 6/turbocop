use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InPatternThen;

impl Cop for InPatternThen {
    fn name(&self) -> &'static str {
        "Style/InPatternThen"
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines = source.lines();

        for (i, line_bytes) in lines.enumerate() {
            let line = match std::str::from_utf8(line_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line.trim();

            // Look for `in <pattern>;` (semicolon after in pattern)
            if !trimmed.starts_with("in ") {
                continue;
            }

            // Find semicolon that's not inside a string
            let rest = &trimmed[3..]; // skip "in "
            if let Some(semi_pos) = rest.find(';') {
                // Make sure it's not `in pattern then` already
                let before_semi = &rest[..semi_pos];
                if before_semi.contains(" then") {
                    continue;
                }
                // After the semicolon there should be something (body)
                let after_semi = rest[semi_pos + 1..].trim();
                if after_semi.is_empty() {
                    continue;
                }
                // Find the absolute position of the semicolon
                let line_start = line.find("in ").unwrap_or(0);
                let abs_semi_pos = line_start + 3 + semi_pos;

                let line_num = i + 1;
                diagnostics.push(self.diagnostic(
                    source,
                    line_num,
                    abs_semi_pos,
                    format!("Do not use `{}`. Use `{} then` instead.", trimmed.split(';').next().unwrap_or(trimmed), trimmed.split(';').next().unwrap_or(trimmed)),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InPatternThen, "cops/style/in_pattern_then");
}
