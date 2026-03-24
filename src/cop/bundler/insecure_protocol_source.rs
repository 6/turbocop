use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct InsecureProtocolSource;

/// ## Extended corpus investigation (2026-03-19)
///
/// FN=1 from repo `openstack__puppet-swift` — file named `.gemfile` (dotfile)
/// containing `source :rubygems`. Rust's `Path::extension()` returns `None` for
/// dotfiles, so `is_ruby_file()` in `fs.rs` failed to recognize `.gemfile` as a
/// Ruby file. The file was never discovered during directory walking, so no cops
/// could run on it. Fix: added dotfile extension check to `is_ruby_file()`.
impl Cop for InsecureProtocolSource {
    fn name(&self) -> &'static str {
        "Bundler/InsecureProtocolSource"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_http = config.get_bool("AllowHttpProtocol", true);

        for (i, line) in source.lines().enumerate() {
            let line_str = std::str::from_utf8(line).unwrap_or("");
            let trimmed = line_str.trim();
            let line_num = i + 1;

            if !trimmed.starts_with("source ") && !trimmed.starts_with("source(") {
                continue;
            }

            // Check for deprecated symbols
            let deprecated_symbols = [":gemcutter", ":rubygems", ":rubyforge"];
            for sym in &deprecated_symbols {
                if trimmed.contains(sym) {
                    // Find column of the symbol
                    let col = line_str.find(sym).unwrap_or(0);
                    let mut diag = self.diagnostic(
                        source,
                        line_num,
                        col,
                        format!(
                            "The source `{}` is deprecated because HTTP requests are insecure. Please change your source to 'https://rubygems.org' if possible, or 'http://rubygems.org' if not.",
                            sym
                        ),
                    );
                    if let Some(ref mut corr) = corrections {
                        if let Some(line_start) = source.line_col_to_offset(line_num, 0) {
                            let sym_start = line_start + col;
                            let sym_end = sym_start + sym.len();
                            corr.push(crate::correction::Correction {
                                start: sym_start,
                                end: sym_end,
                                replacement: "'https://rubygems.org'".to_string(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                    }
                    diagnostics.push(diag);
                }
            }

            // Check for http:// URLs when AllowHttpProtocol is false
            if !allow_http {
                // Find http:// URLs in source declarations
                if let Some(http_idx) = trimmed.find("'http://") {
                    // Extract the full URL
                    let url_start = http_idx + 1; // skip the quote
                    let rest = &trimmed[url_start..];
                    let url_end = rest.find('\'').unwrap_or(rest.len());
                    let url = &rest[..url_end];
                    let https_url = url.replacen("http://", "https://", 1);
                    let col = line_str.find("'http://").unwrap_or(0);
                    let mut diag = self.diagnostic(
                        source,
                        line_num,
                        col,
                        format!("Use `{}` instead of `{}`.", https_url, url),
                    );
                    if let Some(ref mut corr) = corrections {
                        if let Some(line_start) = source.line_col_to_offset(line_num, 0) {
                            // Replace just "http://" with "https://" inside the URL
                            let http_col = line_str.find("http://").unwrap_or(0);
                            let abs_start = line_start + http_col;
                            corr.push(crate::correction::Correction {
                                start: abs_start,
                                end: abs_start + 7, // "http://" is 7 bytes
                                replacement: "https://".to_string(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                    }
                    diagnostics.push(diag);
                } else if let Some(http_idx) = trimmed.find("\"http://") {
                    let url_start = http_idx + 1;
                    let rest = &trimmed[url_start..];
                    let url_end = rest.find('"').unwrap_or(rest.len());
                    let url = &rest[..url_end];
                    let https_url = url.replacen("http://", "https://", 1);
                    let col = line_str.find("\"http://").unwrap_or(0);
                    let mut diag = self.diagnostic(
                        source,
                        line_num,
                        col,
                        format!("Use `{}` instead of `{}`.", https_url, url),
                    );
                    if let Some(ref mut corr) = corrections {
                        if let Some(line_start) = source.line_col_to_offset(line_num, 0) {
                            let http_col = line_str.find("http://").unwrap_or(0);
                            let abs_start = line_start + http_col;
                            corr.push(crate::correction::Correction {
                                start: abs_start,
                                end: abs_start + 7,
                                replacement: "https://".to_string(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                    }
                    diagnostics.push(diag);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        InsecureProtocolSource,
        "cops/bundler/insecure_protocol_source"
    );
    crate::cop_autocorrect_fixture_tests!(
        InsecureProtocolSource,
        "cops/bundler/insecure_protocol_source"
    );

    #[test]
    fn autocorrect_deprecated_symbol() {
        let input = b"source :rubygems\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&InsecureProtocolSource, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"source 'https://rubygems.org'\n");
    }

    #[test]
    fn autocorrect_all_deprecated_symbols() {
        let input = b"source :gemcutter\nsource :rubyforge\n";
        let (diags, corrections) =
            crate::testutil::run_cop_autocorrect(&InsecureProtocolSource, input);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.corrected));
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(
            corrected,
            b"source 'https://rubygems.org'\nsource 'https://rubygems.org'\n"
        );
    }

    #[test]
    fn autocorrect_http_url() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("AllowHttpProtocol".into(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let input = b"source 'http://rubygems.org'\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect_with_config(
            &InsecureProtocolSource,
            input,
            config,
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"source 'https://rubygems.org'\n");
    }
}
