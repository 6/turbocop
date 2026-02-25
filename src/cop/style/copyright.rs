use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use regex::Regex;

pub struct Copyright;

impl Cop for Copyright {
    fn name(&self) -> &'static str {
        "Style/Copyright"
    }

    fn default_enabled(&self) -> bool {
        false // Matches vendor config/default.yml: Enabled: false
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let notice_pattern = config.get_str("Notice", r"^Copyright (\(c\) )?2[0-9]{3} .+");
        let autocorrect_notice = config.get_str("AutocorrectNotice", "");

        // RuboCop raises a Warning exception in verify_autocorrect_notice! when
        // AutocorrectNotice is empty, which prevents any offense from being added.
        // Match that behavior: no offenses when AutocorrectNotice is not configured.
        if autocorrect_notice.is_empty() {
            return;
        }

        let regex = match Regex::new(notice_pattern) {
            Ok(r) => r,
            Err(_) => return,
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
                    return;
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
                return;
            }
        }

        // No copyright notice found
        diagnostics.push(self.diagnostic(
            source,
            1,
            0,
            format!(
                "Include a copyright notice matching `{}` before any code.",
                notice_pattern
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use std::collections::HashMap;

    /// Build a CopConfig with a non-empty AutocorrectNotice so the cop actually runs.
    /// RuboCop requires this to be set; with an empty value the cop silently skips.
    fn config_with_autocorrect_notice() -> CopConfig {
        CopConfig {
            options: HashMap::from([(
                "AutocorrectNotice".to_string(),
                serde_yml::Value::String("# Copyright (c) 2024 Acme Inc.".to_string()),
            )]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn missing_notice() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/offense/missing_notice.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn missing_notice_with_code() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/offense/missing_notice_with_code.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn missing_notice_wrong_text() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/offense/missing_notice_wrong_text.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &Copyright,
            include_bytes!("../../../tests/fixtures/cops/style/copyright/no_offense.rb"),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn empty_autocorrect_notice_produces_no_offenses() {
        // When AutocorrectNotice is empty (the default), RuboCop raises a Warning
        // in verify_autocorrect_notice! which prevents any offense. We match that
        // behavior by returning early with no diagnostics.
        let diagnostics = crate::testutil::run_cop_full_with_config(
            &Copyright,
            b"# no copyright here\nclass Foo; end\n",
            CopConfig::default(),
        );
        assert!(
            diagnostics.is_empty(),
            "Expected no offenses with empty AutocorrectNotice, got: {:?}",
            diagnostics,
        );
    }
}
