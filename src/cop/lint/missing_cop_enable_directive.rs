use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct MissingCopEnableDirective;

impl Cop for MissingCopEnableDirective {
    fn name(&self) -> &'static str {
        "Lint/MissingCopEnableDirective"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let max_range = get_max_range_size(config);
        // Track open disables: cop_name -> (line_number, column)
        let mut open_disables: HashMap<String, (usize, usize)> = HashMap::new();
        let lines: Vec<&[u8]> = source.lines().collect();

        let mut byte_offset = 0usize;
        for (i, line) in lines.iter().enumerate() {
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

            // Find rubocop directives
            let Some((action, cops, col)) = parse_directive(line_str) else {
                byte_offset += line.len() + 1;
                continue;
            };

            match action {
                "disable" | "todo" => {
                    // Check if this is an inline disable (code before the comment)
                    let before = &line_str[..col];
                    let is_inline = !before.trim().is_empty();
                    if !is_inline {
                        for cop in &cops {
                            open_disables.insert(cop.to_string(), (i + 1, col));
                        }
                    }
                }
                "enable" => {
                    for cop in &cops {
                        if let Some((start_line, _)) = open_disables.remove(cop.as_str()) {
                            // Check if range exceeds MaximumRangeSize
                            if max_range.is_finite() {
                                let range_size = (i + 1) - start_line - 1; // lines between disable and enable
                                if range_size > max_range as usize {
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        start_line,
                                        0,
                                        format_message(cop, Some(max_range as usize)),
                                    ));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            byte_offset += line.len() + 1;
        }

        // Report all remaining open disables (never re-enabled)
        for (cop, (line, _col)) in &open_disables {
            if max_range.is_finite() {
                let range_size = lines.len().saturating_sub(*line);
                if range_size > max_range as usize {
                    diagnostics.push(self.diagnostic(
                        source,
                        *line,
                        0,
                        format_message(cop, Some(max_range as usize)),
                    ));
                }
            } else {
                diagnostics.push(self.diagnostic(
                    source,
                    *line,
                    0,
                    format_message(cop, None),
                ));
            }
        }

        // Sort by line number for deterministic output
        diagnostics.sort_by_key(|d| d.location.line);
    }
}

fn format_message(cop: &str, max_range: Option<usize>) -> String {
    // Determine if it's a department (no `/`) or a specific cop
    let kind = if cop.contains('/') { "cop" } else { "department" };
    match max_range {
        Some(n) => format!(
            "Re-enable {} {} within {} lines after disabling it.",
            cop, kind, n
        ),
        None => format!(
            "Re-enable {} {} with `# rubocop:enable` after disabling it.",
            cop, kind,
        ),
    }
}

fn get_max_range_size(config: &CopConfig) -> f64 {
    config
        .options
        .get("MaximumRangeSize")
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_u64().map(|u| u as f64))
                .or_else(|| {
                    v.as_str().and_then(|s| {
                        if s == ".inf" || s == "Infinity" {
                            Some(f64::INFINITY)
                        } else {
                            s.parse::<f64>().ok()
                        }
                    })
                })
        })
        .unwrap_or(f64::INFINITY)
}

fn parse_directive(line: &str) -> Option<(&str, Vec<String>, usize)> {
    let hash_pos = line.find('#')?;
    let after_hash = &line[hash_pos + 1..].trim_start();

    if !after_hash.starts_with("rubocop:") {
        return None;
    }

    let after_prefix = &after_hash["rubocop:".len()..];

    // Extract action
    let action_end = after_prefix
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(after_prefix.len());
    let action = &after_prefix[..action_end];

    if !matches!(action, "disable" | "enable" | "todo") {
        return None;
    }

    let cops_str = after_prefix[action_end..].trim();
    // Strip `-- comment` suffix
    let cops_str = match cops_str.find(" -- ") {
        Some(idx) => &cops_str[..idx],
        None => cops_str,
    };

    let cops: Vec<String> = cops_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // We need to return a reference to the action string within the original line.
    // Since we trimmed, use the computed action.
    // But we can't return a reference to a substring we created.
    // Let's compute the action from the original line instead.
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
    crate::cop_scenario_fixture_tests!(
        MissingCopEnableDirective,
        "cops/lint/missing_cop_enable_directive",
        missing_enable_cop = "missing_enable_cop.rb",
        missing_enable_dept = "missing_enable_dept.rb",
        missing_enable_two = "missing_enable_two.rb",
    );
}
