use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Checks for `# rubocop:enable` comments that can be removed because
/// the cop was not previously disabled.
pub struct RedundantCopEnableDirective;

impl Cop for RedundantCopEnableDirective {
    fn name(&self) -> &'static str {
        "Lint/RedundantCopEnableDirective"
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
        // Track which cops/departments are currently disabled
        let mut disabled: HashSet<String> = HashSet::new();

        let mut byte_offset = 0usize;
        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => {
                    byte_offset += line.len() + 1;
                    continue;
                }
            };

            // Skip lines inside strings/heredocs
            if let Some(hash_pos) = line_str.find('#') {
                if !code_map.is_not_string(byte_offset + hash_pos) {
                    byte_offset += line.len() + 1;
                    continue;
                }
            }

            let directives = parse_all_directives(line_str);
            if directives.is_empty() {
                byte_offset += line.len() + 1;
                continue;
            }

            for (action, cops, _col) in directives {
                match action {
                    "disable" | "todo" => {
                        for cop in &cops {
                            disabled.insert(cop.to_string());
                        }
                    }
                    "enable" => {
                        for cop in &cops {
                            if cop == "all" {
                                // `enable all` is redundant only if nothing was disabled
                                if disabled.is_empty() {
                                    let col = find_cop_column(line_str, cop);
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        i + 1,
                                        col,
                                        "Unnecessary enabling of all cops.".to_string(),
                                    ));
                                } else {
                                    disabled.clear();
                                }
                                continue;
                            }

                            let was_disabled = disabled.remove(cop.as_str());

                            // Also check if a department enable covers this cop
                            let dept = cop.split('/').next().unwrap_or(cop);
                            let dept_was_disabled = if dept != cop.as_str() {
                                disabled.contains(dept)
                            } else {
                                false
                            };

                            if !was_disabled && !dept_was_disabled {
                                let col = find_cop_column(line_str, cop);
                                diagnostics.push(self.diagnostic(
                                    source,
                                    i + 1,
                                    col,
                                    format!("Unnecessary enabling of {}.", cop),
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }

            byte_offset += line.len() + 1;
        }

    }
}

fn find_cop_column(line: &str, cop: &str) -> usize {
    // Find the position of the cop name in the enable directive
    if let Some(enable_pos) = line.find("rubocop:enable") {
        let after_enable = &line[enable_pos + "rubocop:enable".len()..];
        if let Some(cop_pos) = after_enable.find(cop) {
            return enable_pos + "rubocop:enable".len() + cop_pos;
        }
    }
    0
}

/// Parse all rubocop directives in a line. A line may contain multiple
/// directives (e.g., in doc comments with embedded examples).
/// Returns a list of (action, cops, col) tuples.
fn parse_all_directives(line: &str) -> Vec<(&str, Vec<String>, usize)> {
    let mut results = Vec::new();
    let mut search_from = 0;

    while search_from < line.len() {
        let remaining = &line[search_from..];
        let Some(rubocop_pos) = remaining.find("rubocop:") else {
            break;
        };

        let abs_pos = search_from + rubocop_pos;

        // A real rubocop directive has the form `# rubocop:action` where
        // the `#` is the Ruby comment marker (possibly preceded only by code
        // or whitespace), not part of documentation text. We require the `#`
        // immediately before `rubocop:` (with only whitespace between) AND
        // that `#` must be either at the start of the line (after whitespace)
        // or preceded by code (inline comment). Documentation examples like
        // `` `# rubocop:enable` `` inside comments are excluded because the
        // `#` before `rubocop:` is preceded by a backtick character.
        let before = &line[..abs_pos];
        let before_trimmed = before.trim_end();
        if !before_trimmed.ends_with('#') {
            search_from = abs_pos + "rubocop:".len();
            continue;
        }
        // The `#` must be the comment-starting hash, not an example in backticks.
        // Check that the character before `#` is whitespace, start-of-line, or a
        // code character (not a backtick which indicates an embedded example).
        let hash_pos = before_trimmed.len() - 1;
        if hash_pos > 0 {
            let char_before_hash = before_trimmed.as_bytes()[hash_pos - 1];
            if char_before_hash == b'`' {
                search_from = abs_pos + "rubocop:".len();
                continue;
            }
        }

        let after_prefix = &remaining[rubocop_pos + "rubocop:".len()..];

        let action_end = after_prefix
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(after_prefix.len());
        let action = &after_prefix[..action_end];

        if !matches!(action, "disable" | "enable" | "todo") {
            search_from = abs_pos + "rubocop:".len();
            continue;
        }

        let cops_str = after_prefix[action_end..].trim();
        // Stop at next # rubocop: directive or end of string
        let cops_str = match cops_str.find(" -- ") {
            Some(idx) => &cops_str[..idx],
            None => cops_str,
        };
        // Also stop at next # rubocop: sequence
        let cops_str = match cops_str.find("# rubocop:") {
            Some(idx) => &cops_str[..idx],
            None => cops_str,
        };

        let cops: Vec<String> = cops_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let action_str = match action {
            "disable" => "disable",
            "enable" => "enable",
            "todo" => "todo",
            _ => {
                search_from = abs_pos + "rubocop:".len();
                continue;
            }
        };

        results.push((action_str, cops, abs_pos));

        // Move past this directive
        search_from = abs_pos + "rubocop:".len() + action_end + cops_str.len();
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantCopEnableDirective, "cops/lint/redundant_cop_enable_directive");
}
