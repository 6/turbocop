use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::path::{Component, Path};

/// Checks for usage of `%q`/`%Q` when normal quotes would do.
///
/// The corpus misses were static `%Q` strings that merely contained `"` or
/// spanned multiple lines. RuboCop still flags those unless the full `%Q`
/// source either contains both quote kinds, is dynamic and contains `"`, or
/// is a static literal whose source really requires double quotes (for example
/// because of a single quote or a non-quote escape). Matching against the full
/// node source also preserves `%q` behavior for strings whose quote mix is not
/// visible in Prism's extracted content slice.
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
            let node_source = string_node.location().as_slice();

            if opening.starts_with(b"%q") {
                if contains_single_and_double_quotes(node_source)
                    || acceptable_percent_q(node_source)
                {
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
                if contains_single_and_double_quotes(node_source)
                    || acceptable_static_percent_capital_q(node_source)
                {
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

            let node_source = node.location().as_slice();
            if contains_single_and_double_quotes(node_source)
                || acceptable_dynamic_percent_capital_q(node_source)
            {
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

fn contains_single_and_double_quotes(source: &[u8]) -> bool {
    source.contains(&b'\'') && source.contains(&b'"')
}

fn acceptable_percent_q(source: &[u8]) -> bool {
    contains_interpolation_pattern(source) || has_non_backslash_escape(source)
}

fn acceptable_static_percent_capital_q(source: &[u8]) -> bool {
    source.contains(&b'"') && double_quotes_required(source)
}

fn acceptable_dynamic_percent_capital_q(source: &[u8]) -> bool {
    source.contains(&b'"') && contains_interpolation_pattern(source)
}

/// Check if the source contains escape sequences other than just `\\`.
fn has_non_backslash_escape(source: &[u8]) -> bool {
    let mut i = 0;
    while i < source.len() {
        if source[i] == b'\\' && i + 1 < source.len() {
            if source[i + 1] != b'\\' {
                return true;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    false
}

/// Check if the source contains a string interpolation pattern `#{...}`.
fn contains_interpolation_pattern(source: &[u8]) -> bool {
    source.windows(2).enumerate().any(|(idx, window)| {
        window == b"#{"
            && source[idx + 2..]
                .iter()
                .position(|&b| b == b'}')
                .is_some_and(|offset| offset > 0)
    })
}

/// Match RuboCop's `double_quotes_required?` helper on the full node source.
fn double_quotes_required(source: &[u8]) -> bool {
    let mut i = 0;

    while i < source.len() {
        match source[i] {
            b'\'' => return true,
            b'\\' => {
                let run_start = i;
                while i < source.len() && source[i] == b'\\' {
                    i += 1;
                }

                let run_len = i - run_start;
                let next = source.get(i).copied();

                if run_len % 2 == 1 && !matches!(next, Some(b'\\' | b'"')) {
                    return true;
                }
            }
            _ => i += 1,
        }
    }

    false
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
