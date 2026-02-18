use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
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

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        // Track which cops/departments are currently disabled
        let mut disabled: HashSet<String> = HashSet::new();

        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let Some((action, cops, _col)) = parse_directive(line_str) else {
                continue;
            };

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
                            if cop.contains('/') {
                                diagnostics.push(self.diagnostic(
                                    source,
                                    i + 1,
                                    col,
                                    format!("Unnecessary enabling of {}.", cop),
                                ));
                            } else {
                                diagnostics.push(self.diagnostic(
                                    source,
                                    i + 1,
                                    col,
                                    format!("Unnecessary enabling of {}.", cop),
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        diagnostics
    }
}

fn find_cop_column(line: &str, cop: &str) -> usize {
    // Find the position of the cop name in the enable directive
    if let Some(enable_pos) = line.find("rubocop:enable") {
        let after_enable = &line[enable_pos + "rubocop:enable".len()..];
        if let Some(cop_pos) = after_enable.find(cop.as_str()) {
            return enable_pos + "rubocop:enable".len() + cop_pos;
        }
    }
    0
}

fn parse_directive(line: &str) -> Option<(&str, Vec<String>, usize)> {
    let hash_pos = line.find('#')?;
    let after_hash = &line[hash_pos + 1..].trim_start();

    if !after_hash.starts_with("rubocop:") {
        return None;
    }

    let after_prefix = &after_hash["rubocop:".len()..];

    let action_end = after_prefix
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(after_prefix.len());
    let action = &after_prefix[..action_end];

    if !matches!(action, "disable" | "enable" | "todo") {
        return None;
    }

    let cops_str = after_prefix[action_end..].trim();
    let cops_str = match cops_str.find(" -- ") {
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
        _ => return None,
    };

    Some((action_str, cops, hash_pos))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantCopEnableDirective, "cops/lint/redundant_cop_enable_directive");
}
