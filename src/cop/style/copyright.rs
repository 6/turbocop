use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use regex::Regex;

pub struct Copyright;

impl Cop for Copyright {
    fn name(&self) -> &'static str {
        "Style/Copyright"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let notice_pattern = config.get_str("Notice", r"^Copyright (\(c\) )?2[0-9]{3} .+");
        let _autocorrect_notice = config.get_str("AutocorrectNotice", "");

        let regex = match Regex::new(notice_pattern) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        // Search all comment lines for the copyright notice
        let lines: Vec<&[u8]> = source.lines().collect();

        for line in &lines {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s.trim(),
                Err(_) => continue,
            };

            if line_str.starts_with('#') {
                let comment_text = line_str.trim_start_matches('#').trim();
                if regex.is_match(comment_text) {
                    return Vec::new();
                }
            }

            // Also check inside block comments
            if line_str.starts_with("=begin") || line_str.starts_with("=end") {
                continue;
            }
            // Check non-comment lines within block comments
            let line_str_raw = match std::str::from_utf8(line) {
                Ok(s) => s.trim(),
                Err(_) => continue,
            };
            if regex.is_match(line_str_raw) {
                return Vec::new();
            }
        }

        // No copyright notice found
        vec![self.diagnostic(
            source,
            1,
            0,
            format!(
                "Include a copyright notice matching `{}` before any code.",
                notice_pattern
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        Copyright, "cops/style/copyright",
        missing_notice = "missing_notice.rb",
        missing_notice_with_code = "missing_notice_with_code.rb",
        missing_notice_wrong_text = "missing_notice_wrong_text.rb",
    );
}
