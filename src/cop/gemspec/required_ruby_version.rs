use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RequiredRubyVersion;

/// Extract the first two digits from a version string and join with '.'.
/// e.g. ">= 2.7.0" → "2.7", "~> 3.4" → "3.4"
fn extract_version_digits(s: &str) -> Option<String> {
    let digits: Vec<char> = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 2 {
        Some(format!("{}.{}", digits[0], digits[1]))
    } else {
        None
    }
}

/// Format a TargetRubyVersion f64 as "X.Y".
fn format_target_version(v: f64) -> String {
    let major = v as u32;
    let minor = ((v * 10.0).round() as u32) % 10;
    format!("{major}.{minor}")
}

impl Cop for RequiredRubyVersion {
    fn name(&self) -> &'static str {
        "Gemspec/RequiredRubyVersion"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut found = false;
        let mut version_info: Option<(usize, usize, String)> = None; // (line, col, version_str)

        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            if trimmed.starts_with('#') {
                continue;
            }

            // Check for required_ruby_version assignment
            if trimmed.contains(".required_ruby_version") {
                let after = trimmed.split(".required_ruby_version").nth(1).unwrap_or("");
                let after_trimmed = after.trim_start();
                // Must be an assignment (= or >=) not just a method call check
                if after_trimmed.starts_with('=') || after_trimmed.is_empty() {
                    found = true;

                    // Try to extract the version string for mismatch checking.
                    // Look for a quoted string after the '=' sign.
                    if let Some(eq_pos) = after_trimmed.find('=') {
                        let rhs = &after_trimmed[eq_pos + 1..];
                        // Find the first quoted string (single or double)
                        let quote_char = rhs
                            .find(['\'', '"'])
                            .map(|p| (p, rhs.as_bytes()[p] as char));
                        if let Some((_start, qc)) = quote_char {
                            let after_open = &rhs[_start + 1..];
                            if let Some(end) = after_open.find(qc) {
                                let ver_str = &after_open[..end];
                                if let Some(extracted) = extract_version_digits(ver_str) {
                                    // Calculate column: find the quoted string position in the original line
                                    let ver_literal = &rhs[_start..=_start + 1 + end];
                                    let col = line_str.find(ver_literal).unwrap_or(0);
                                    version_info = Some((line_idx + 1, col, extracted));
                                }
                            }
                        }
                    }

                    break;
                }
            }
        }

        if !found {
            diagnostics.push(self.diagnostic(
                source,
                1,
                0,
                "`required_ruby_version` should be set in gemspec.".to_string(),
            ));
            return;
        }

        // Check version mismatch against TargetRubyVersion
        if let Some((line, col, ref gemspec_version)) = version_info {
            let target = config
                .options
                .get("TargetRubyVersion")
                .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)));
            if let Some(target_ver) = target {
                let target_str = format_target_version(target_ver);
                if *gemspec_version != target_str {
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        format!(
                            "`required_ruby_version` and `TargetRubyVersion` \
                             ({target_str}, which may be specified in .rubocop.yml) should be equal."
                        ),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    crate::cop_scenario_fixture_tests!(
        RequiredRubyVersion,
        "cops/gemspec/required_ruby_version",
        missing_version = "missing_version.rb",
        empty_gemspec = "empty_gemspec.rb",
        only_other_attrs = "only_other_attrs.rb",
    );

    fn config_with_target_ruby(version: f64) -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "TargetRubyVersion".to_string(),
            serde_yml::Value::Number(serde_yml::value::Number::from(version)),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn version_mismatch() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/offense/version_mismatch.rb"
            ),
            config_with_target_ruby(3.1),
        );
    }

    #[test]
    fn version_match_no_offense() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RequiredRubyVersion,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/required_ruby_version/no_offense.rb"
            ),
            config_with_target_ruby(3.0),
        );
    }
}
