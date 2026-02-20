use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct InsecureProtocolSource;

impl Cop for InsecureProtocolSource {
    fn name(&self) -> &'static str {
        "Bundler/InsecureProtocolSource"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, _corrections: Option<&mut Vec<crate::correction::Correction>>) {
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
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        col,
                        format!(
                            "The source `{}` is deprecated because HTTP requests are insecure. Please change your source to 'https://rubygems.org' if possible, or 'http://rubygems.org' if not.",
                            sym
                        ),
                    ));
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
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        col,
                        format!("Use `{}` instead of `{}`.", https_url, url),
                    ));
                } else if let Some(http_idx) = trimmed.find("\"http://") {
                    let url_start = http_idx + 1;
                    let rest = &trimmed[url_start..];
                    let url_end = rest.find('"').unwrap_or(rest.len());
                    let url = &rest[..url_end];
                    let https_url = url.replacen("http://", "https://", 1);
                    let col = line_str.find("\"http://").unwrap_or(0);
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        col,
                        format!("Use `{}` instead of `{}`.", https_url, url),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InsecureProtocolSource, "cops/bundler/insecure_protocol_source");
}
