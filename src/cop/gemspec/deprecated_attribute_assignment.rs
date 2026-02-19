use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DeprecatedAttributeAssignment;

const DEPRECATED_ATTRS: &[&str] = &[
    "test_files",
    "date",
    "specification_version",
    "rubygems_version",
];

impl Cop for DeprecatedAttributeAssignment {
    fn name(&self) -> &'static str {
        "Gemspec/DeprecatedAttributeAssignment"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>) {
        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            // Skip comment lines
            if trimmed.starts_with('#') {
                continue;
            }
            for &attr in DEPRECATED_ATTRS {
                // Look for `.attr` followed by `=` or ` =`
                let pattern = format!(".{attr}");
                if let Some(pos) = trimmed.find(&pattern) {
                    let after = &trimmed[pos + pattern.len()..].trim_start();
                    if after.starts_with('=') || after.is_empty() {
                        // Find position in original line
                        let orig_pos = line_str.find(&pattern).unwrap();
                        diagnostics.push(self.diagnostic(
                            source,
                            line_idx + 1,
                            orig_pos + 1, // skip the dot
                            format!("Do not set `{attr}` in gemspec."),
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
    crate::cop_fixture_tests!(
        DeprecatedAttributeAssignment,
        "cops/gemspec/deprecated_attribute_assignment"
    );
}
