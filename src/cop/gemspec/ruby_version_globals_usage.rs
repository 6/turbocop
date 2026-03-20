use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RubyVersionGlobalsUsage;

/// Returns true if position `pos` is inside a string literal but NOT inside a `#{}` interpolation.
fn is_inside_string(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    let mut interp_depth = 0;
    let mut i = 0;
    while i < pos && i < bytes.len() {
        if in_double && interp_depth > 0 {
            match bytes[i] {
                b'{' => interp_depth += 1,
                b'}' => interp_depth -= 1,
                _ => {}
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            b'\\' if in_single || in_double => {
                i += 1; // skip escaped character
            }
            b'#' if in_double && i + 1 < bytes.len() && bytes[i + 1] == b'{' => {
                interp_depth = 1;
                i += 2;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    // Inside a string but not inside an interpolation block
    in_single || (in_double && interp_depth == 0)
}

impl Cop for RubyVersionGlobalsUsage {
    fn name(&self) -> &'static str {
        "Gemspec/RubyVersionGlobalsUsage"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
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
                // Skip if RUBY_VERSION is inside a string literal
                let in_string = is_inside_string(line_str, abs_pos);
                if before_ok && after_ok && !in_string {
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
