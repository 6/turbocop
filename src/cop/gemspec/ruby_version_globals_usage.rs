use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RubyVersionGlobalsUsage;

impl Cop for RubyVersionGlobalsUsage {
    fn name(&self) -> &'static str {
        "Gemspec/RubyVersionGlobalsUsage"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, _corrections: Option<&mut Vec<crate::correction::Correction>>) {
        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            // Skip comment lines
            if line_str.trim_start().starts_with('#') {
                continue;
            }
            // Find all occurrences of RUBY_VERSION in the line
            let mut search_from = 0;
            while let Some(pos) = line_str[search_from..].find("RUBY_VERSION") {
                let abs_pos = search_from + pos;
                // Ensure it's not part of a larger identifier
                let before_ok = abs_pos == 0
                    || !line_str.as_bytes()[abs_pos - 1].is_ascii_alphanumeric()
                        && line_str.as_bytes()[abs_pos - 1] != b'_';
                let after_pos = abs_pos + "RUBY_VERSION".len();
                let after_ok = after_pos >= line_str.len()
                    || !line_str.as_bytes()[after_pos].is_ascii_alphanumeric()
                        && line_str.as_bytes()[after_pos] != b'_';
                if before_ok && after_ok {
                    diagnostics.push(self.diagnostic(
                        source,
                        line_idx + 1,
                        abs_pos,
                        "Do not use `RUBY_VERSION` in gemspec.".to_string(),
                    ));
                }
                search_from = abs_pos + "RUBY_VERSION".len();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RubyVersionGlobalsUsage,
        "cops/gemspec/ruby_version_globals_usage"
    );
}
