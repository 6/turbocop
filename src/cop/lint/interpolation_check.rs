use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct InterpolationCheck;

impl Cop for InterpolationCheck {
    fn name(&self) -> &'static str {
        "Lint/InterpolationCheck"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (i, line) in source.lines().enumerate() {
            // Look for single-quoted strings containing #{
            // Simple heuristic: find '...' containing #{
            let mut pos = 0;
            while pos < line.len() {
                if line[pos] == b'\'' {
                    // Found opening single quote, find closing
                    let start = pos;
                    pos += 1;
                    let mut found_interp = false;
                    let mut interp_col = 0;
                    while pos < line.len() && line[pos] != b'\'' {
                        if pos + 1 < line.len() && line[pos] == b'#' && line[pos + 1] == b'{' {
                            if !found_interp {
                                interp_col = pos;
                                found_interp = true;
                            }
                        }
                        pos += 1;
                    }
                    if pos < line.len() && line[pos] == b'\'' && found_interp {
                        // We found a single-quoted string with #{ inside
                        // But make sure this isn't inside a double-quoted string or comment
                        // Simple check: the quote at `start` should not be preceded by a backslash
                        let escaped = start > 0 && line[start - 1] == b'\\';
                        if !escaped {
                            diagnostics.push(self.diagnostic(
                                source,
                                i + 1,
                                interp_col,
                                "Interpolation in single-quoted string detected. Did you mean to use double quotes?".to_string(),
                            ));
                        }
                    }
                    if pos < line.len() {
                        pos += 1;
                    }
                } else if line[pos] == b'#' {
                    // Rest of line is a comment, stop scanning
                    break;
                } else if line[pos] == b'"' {
                    // Skip double-quoted strings
                    pos += 1;
                    while pos < line.len() && line[pos] != b'"' {
                        if line[pos] == b'\\' && pos + 1 < line.len() {
                            pos += 1; // skip escaped char
                        }
                        pos += 1;
                    }
                    if pos < line.len() {
                        pos += 1;
                    }
                } else {
                    pos += 1;
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InterpolationCheck, "cops/lint/interpolation_check");
}
