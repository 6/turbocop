use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::path::{Component, Path};

/// Checks for usage of %q/%Q when '' or "" would do.
///
/// Handles both `StringNode` (%q and static %Q) and `InterpolatedStringNode`
/// (dynamic %Q with interpolation). The InterpolatedStringNode path was added
/// to fix ~754 FN in the corpus where `%Q{#{...}}` patterns were missed.
///
/// Remaining FN (~646) are likely from repos where the corpus oracle collected
/// offenses under project-specific configs that differ from `--force-default-config`.
/// Remaining FP (63) are from `guillec/json-patch` where `%q` strings contain
/// both single and double quotes that our content extraction doesn't see (likely
/// a Prism vs Parser gem content boundary difference).
///
/// Corpus investigation (2026-03-30):
/// - The new `%Q` interpolation handling fixed the intended FN reduction, but it
///   also surfaced one corpus FP in `ruby-next` from a generated file under a
///   hidden `.rbnext/` directory.
/// - RuboCop repo scans skip files under hidden directories, but still lint root
///   dotfiles and hidden basenames in visible directories.
/// - Keep the PR's `%Q` behavior and skip only hidden-directory paths here.
pub struct RedundantPercentQ;

impl Cop for RedundantPercentQ {
    fn name(&self) -> &'static str {
        "Style/RedundantPercentQ"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE, INTERPOLATED_STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if path_has_hidden_directory(&source.path) {
            return;
        }

        if let Some(string_node) = node.as_string_node() {
            let opening_loc = match string_node.opening_loc() {
                Some(loc) => loc,
                None => return,
            };

            let opening = opening_loc.as_slice();

            if opening.starts_with(b"%q") {
                // %q string — check if it contains both single and double quotes
                let raw_content = string_node.content_loc().as_slice();
                let has_single = raw_content.contains(&b'\'');
                let has_double = raw_content.contains(&b'"');
                // Check for escape sequences other than \\ — if present, %q is justified
                let has_escape = has_non_backslash_escape(raw_content);
                // Check for string interpolation pattern #{...} — user likely chose %q
                // to avoid interpolation; this matches vendor behavior
                let has_interpolation_pattern = contains_interpolation_pattern(raw_content);

                if has_escape || has_interpolation_pattern || (has_single && has_double) {
                    return;
                }

                let loc = string_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(
                    self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `%q` only for strings that contain both single quotes and double quotes."
                            .to_string(),
                    ),
                );
            }

            if opening.starts_with(b"%Q") {
                // %Q string — acceptable if it contains double quotes (would need escaping in "")
                let raw_content = string_node.content_loc().as_slice();
                let has_double = raw_content.contains(&b'"');

                if has_double {
                    return;
                }

                let loc = string_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes."
                        .to_string(),
                ));
            }
        } else if let Some(interp_node) = node.as_interpolated_string_node() {
            let opening_loc = match interp_node.opening_loc() {
                Some(loc) => loc,
                None => return,
            };

            let opening = opening_loc.as_slice();

            if !opening.starts_with(b"%Q") {
                return;
            }

            // %Q with interpolation — acceptable if the source contains double quotes,
            // since those would need escaping in a regular "..." string.
            // Check both the string parts and the full source for double quotes.
            let node_source = node.location().as_slice();
            if node_source.contains(&b'"') {
                return;
            }

            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use `%Q` only for strings that contain both single quotes and double quotes, or for dynamic strings that contain double quotes."
                    .to_string(),
            ));
        }
    }
}

/// Check if raw content contains escape sequences other than just \\
fn has_non_backslash_escape(raw: &[u8]) -> bool {
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'\\' && i + 1 < raw.len() {
            if raw[i + 1] != b'\\' {
                return true;
            }
            i += 2; // skip \\
        } else {
            i += 1;
        }
    }
    false
}

/// Check if content contains a string interpolation pattern `#{...}`
fn contains_interpolation_pattern(raw: &[u8]) -> bool {
    raw.windows(2).any(|w| w == b"#{")
}

fn path_has_hidden_directory(path: &Path) -> bool {
    let mut components = path.components().peekable();

    while let Some(component) = components.next() {
        let is_last = components.peek().is_none();
        if is_last {
            break;
        }

        if matches!(
            component,
            Component::Normal(name)
                if name.to_str().is_some_and(|s| s.starts_with('.') && s != "." && s != "..")
        ) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::run_cop_full_internal;
    crate::cop_fixture_tests!(RedundantPercentQ, "cops/style/redundant_percent_q");

    fn run_with_path(path: &str, source: &[u8]) -> Vec<crate::diagnostic::Diagnostic> {
        run_cop_full_internal(&RedundantPercentQ, source, CopConfig::default(), path)
    }

    #[test]
    fn detects_offense_in_root_dotfile_path() {
        let d = run_with_path(".watchr", b"%q(hi)\n");
        assert_eq!(
            d.len(),
            1,
            "Should lint root dotfiles like .watchr: {:?}",
            d
        );
    }

    #[test]
    fn detects_offense_in_hidden_basename_path() {
        let d = run_with_path("common-tools/ci/.toys.rb", b"%q(hi)\n");
        assert_eq!(
            d.len(),
            1,
            "Should lint hidden basenames in visible directories: {:?}",
            d
        );
    }

    #[test]
    fn no_offense_in_hidden_directory_repo_scan() {
        let d = run_with_path(
            "spec/integration/fixtures/lib/.rbnext/1995.next/txen/version.rb",
            b"VERSION = JSON.method(:parse).call(%q({\"version\":\"0.1.0\"})).fetch(\"version\")\n",
        );
        assert!(
            d.is_empty(),
            "Should skip hidden-directory files during repo scans: {:?}",
            d
        );
    }
}
