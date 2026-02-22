use std::sync::LazyLock;

use regex::Regex;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct DepartmentName;

/// Regex matching rubocop directive comments.
/// Captures: (1) = prefix up to and including directive keyword + trailing space,
///           (2) = the directive keyword itself, (3) = the remainder (cop list).
static DIRECTIVE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#\s*rubocop\s*:\s*((?:dis|en)able|todo)\s+(.+)").unwrap());

/// A valid cop/department token: either `Department/CopName` or `all`.
static VALID_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z]+/[A-Za-z]+$").unwrap());

/// Known departments that can be used without a slash.
const KNOWN_DEPARTMENTS: &[&str] = &[
    "Bundler",
    "Gemspec",
    "Layout",
    "Lint",
    "Metrics",
    "Migration",
    "Naming",
    "Performance",
    "Rails",
    "RSpec",
    "Security",
    "Style",
];

/// Returns true if the name contains unexpected characters for a department name.
/// Unexpected = anything other than A-Za-z, `/`, `,`, or space.
fn contains_unexpected_char(name: &str) -> bool {
    name.bytes()
        .any(|b| !b.is_ascii_alphabetic() && b != b'/' && b != b',' && b != b' ')
}

impl Cop for DepartmentName {
    fn name(&self) -> &'static str {
        "Migration/DepartmentName"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut byte_offset: usize = 0;

        for (line_idx, line) in source.lines().enumerate() {
            let line_num = line_idx + 1;
            let line_len = line.len() + 1; // +1 for newline

            let line_str = String::from_utf8_lossy(line);

            let Some(caps) = DIRECTIVE_RE.captures(&line_str) else {
                byte_offset += line_len;
                continue;
            };

            let full_match = caps.get(0).unwrap();

            // Skip directives inside string/heredoc regions (check position of
            // the # character, not just line start, since the line may have
            // code before a string containing `#rubocop:disable`)
            if !code_map.is_not_string(byte_offset + full_match.start()) {
                byte_offset += line_len;
                continue;
            }

            // Skip directives inside documentation comments (nested #).
            // e.g. `#   # rubocop:disable Foo` — the directive is in a YARD example.
            let before_directive = &line_str[..full_match.start()];
            if before_directive.contains('#') {
                byte_offset += line_len;
                continue;
            }

            // Get the byte offset where the cop list starts within the line.
            let cop_list_match = caps.get(2).unwrap();
            // The absolute offset in the line where the match starts
            let match_start_in_line = full_match.start();
            // The offset within the matched region where the cop list starts
            let cop_list_start = cop_list_match.start();
            // Absolute position of cop list in the original line
            let cop_list_abs_start = match_start_in_line + (cop_list_start - full_match.start());

            let cop_list_raw = cop_list_match.as_str();

            // Scan tokens separated by commas. RuboCop scans with /[^,]+|\W+/
            // which effectively splits by comma but also yields whitespace-only tokens.
            let mut offset = cop_list_abs_start;
            for segment in cop_list_raw.split(',') {
                let trimmed = segment.trim();
                let trimmed_start = if trimmed.is_empty() {
                    offset
                } else {
                    // Find the position of trimmed within segment
                    let leading_ws = segment.len() - segment.trim_start().len();
                    offset + leading_ws
                };

                if !trimmed.is_empty()
                    && trimmed != "all"
                    && !VALID_TOKEN_RE.is_match(trimmed)
                    && !KNOWN_DEPARTMENTS.contains(&trimmed)
                {
                    // Check for unexpected characters that should stop processing
                    if contains_unexpected_char(trimmed) {
                        break;
                    }
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        trimmed_start,
                        "Department name is missing.".to_string(),
                    ));
                }

                // Stop if the segment contains unexpected characters (e.g. `--`, `#`)
                if contains_unexpected_char(segment) {
                    break;
                }

                offset += segment.len() + 1; // +1 for the comma
            }

            byte_offset += line_len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(DepartmentName, "cops/migration/department_name");

    #[test]
    fn detects_missing_department_in_disable() {
        let diags = run_cop_full(&DepartmentName, b"x = 1 # rubocop:disable Alias\n");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "Department name is missing.");
        assert_eq!(diags[0].cop_name, "Migration/DepartmentName");
    }

    #[test]
    fn accepts_qualified_cop_name() {
        let diags = run_cop_full(&DepartmentName, b"x = 1 # rubocop:disable Style/Alias\n");
        assert!(diags.is_empty());
    }

    #[test]
    fn accepts_all_keyword() {
        let diags = run_cop_full(&DepartmentName, b"x = 1 # rubocop:disable all\n");
        assert!(diags.is_empty());
    }

    #[test]
    fn accepts_department_name_alone() {
        let diags = run_cop_full(
            &DepartmentName,
            b"# rubocop:disable Style\nalias :ala :bala\n",
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn stops_at_unexpected_characters() {
        let diags = run_cop_full(
            &DepartmentName,
            b"# rubocop:disable Style/Alias -- because something\nalias :ala :bala\n",
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn handles_spaces_around_colon() {
        let diags = run_cop_full(
            &DepartmentName,
            b"# rubocop : todo Alias, LineLength\nalias :ala :bala\n",
        );
        assert_eq!(diags.len(), 2);
    }

    #[test]
    fn severity_is_warning() {
        assert_eq!(DepartmentName.default_severity(), Severity::Warning);
    }

    #[test]
    fn skip_directives_in_heredoc() {
        let diags = run_cop_full(
            &DepartmentName,
            b"x = <<~RUBY\n  # rubocop:disable Alias\nRUBY\n",
        );
        assert!(
            diags.is_empty(),
            "Should not fire on directives inside heredoc"
        );
    }

    #[test]
    fn skip_directives_in_string_literal() {
        let diags = run_cop_full(
            &DepartmentName,
            b"let(:text) { '#rubocop:enable Foo, Baz' }\n",
        );
        assert!(
            diags.is_empty(),
            "Should not fire on directives inside string literal"
        );
    }
}
