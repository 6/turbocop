use std::collections::{HashMap, HashSet};

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// **Round 1** — Corpus oracle reported FP=19, FN=2.
/// The core range tracking was correct, but directive parsing was too literal.
/// Real-world code uses both `rubocop:disable` and `rubocop: disable`, and
/// disable comments often carry a trailing explanation. Our parser kept that
/// explanatory text as part of the cop name, so later clean `rubocop:enable`
/// directives did not match and we reported missing enables incorrectly.
/// Fix improved the cop from 158 offenses to 146, leaving 5-6 excess FPs.
///
/// **Round 2** — FP=6, FN=0. Root cause: `# rubocop:enable all` was not
/// recognized as closing all individually opened disables. RuboCop expands
/// `enable all` to re-enable every currently-disabled cop. Our code treated
/// "all" as a literal cop name and only removed the "all" key from
/// `open_disables`, leaving individually disabled cops (e.g.
/// `Metrics/MethodLength`, `Layout`) unclosed. Fix: when the enable directive
/// contains "all", drain all open disables (checking MaximumRangeSize for each).
///
/// **Round 3** — FP=2, FN=0. Root cause: `--` trailing comment marker not
/// handled. RuboCop uses `--` as a delimiter between cop names and explanatory
/// text (e.g., `# rubocop:disable Style/Foo -- use bar, baz instead`). Our
/// `parse_directive` split the entire text after the action on commas without
/// first stripping the `--` suffix, so explanations containing commas produced
/// phantom cop names (e.g., `baz` from `bar, baz instead`). These phantom
/// disables had no matching enable and were reported as FPs. Fix: strip text
/// after `--` before splitting on commas.
///
/// **Round 4** — FP=1, FN=0. Root cause: malformed tokens such as `Metrics/`
/// were still accepted as cop names. In real-world directives like
/// `# rubocop:disable /BlockLength, Metrics/`, RuboCop ignores both malformed
/// tokens entirely, so no later enable is required. Fix: reject tokens that do
/// not start with an alphanumeric character or that end with `/`.
///
/// **Round 5** — FP=3, FN=3. RuboCop treats directive parsing as a valid-prefix
/// scan, not a blind comma split. That means `# rubocop:disable Metrics/`
/// still disables the `Metrics` department, while malformed mixed directives
/// like `# rubocop:disable /BlockLength, Metrics/` disable nothing because the
/// first token is invalid. RuboCop also keeps only the leading valid cop list
/// and ignores freeform trailing text for this cop, even when
/// `Lint/CopDirectiveSyntax` would flag the directive separately. Fixes here:
/// parse only the leading valid cop prefix, ignore inline `# rubocop:enable`
/// comments when closing a multi-line disable, use the directive's actual
/// column, and dedupe multiple missing cops on the same directive location so
/// we match RuboCop's single reported offense per comment range.
pub struct MissingCopEnableDirective;

#[derive(Clone)]
struct OpenDisable {
    line: usize,
    col: usize,
    order: usize,
}

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
        // Track open disables: cop_name -> directive location and insertion order.
        let mut open_disables: HashMap<String, OpenDisable> = HashMap::new();
        let mut disable_order = 0usize;
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
                            open_disables.insert(
                                cop.to_string(),
                                OpenDisable {
                                    line: i + 1,
                                    col,
                                    order: disable_order,
                                },
                            );
                            disable_order += 1;
                        }
                    }
                }
                "enable" => {
                    let before = &line_str[..col];
                    let is_inline = !before.trim().is_empty();
                    if is_inline {
                        byte_offset += line.len() + 1;
                        continue;
                    }

                    let mut closed: Vec<(String, OpenDisable)> =
                        if cops.iter().any(|cop| cop == "all") {
                            open_disables.drain().collect()
                        } else {
                            cops.iter()
                                .filter_map(|cop| {
                                    open_disables
                                        .remove(cop.as_str())
                                        .map(|info| (cop.clone(), info))
                                })
                                .collect()
                        };

                    if max_range.is_finite() {
                        let max_range = max_range as usize;
                        closed.retain(|(_, info)| (i + 1) - info.line - 1 > max_range);
                        push_unique_diagnostics(self, source, diagnostics, closed, Some(max_range));
                    }
                }
                _ => {}
            }

            byte_offset += line.len() + 1;
        }

        // Report all remaining open disables (never re-enabled).
        let mut remaining: Vec<(String, OpenDisable)> = open_disables.into_iter().collect();
        if max_range.is_finite() {
            let max_range = max_range as usize;
            remaining.retain(|(_, info)| lines.len().saturating_sub(info.line) > max_range);
            push_unique_diagnostics(self, source, diagnostics, remaining, Some(max_range));
        } else {
            push_unique_diagnostics(self, source, diagnostics, remaining, None);
        }

        // Sort by location for deterministic output.
        diagnostics.sort_by(|a, b| {
            (a.location.line, a.location.column, a.message.as_str()).cmp(&(
                b.location.line,
                b.location.column,
                b.message.as_str(),
            ))
        });
    }
}

fn push_unique_diagnostics(
    cop: &MissingCopEnableDirective,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    mut entries: Vec<(String, OpenDisable)>,
    max_range: Option<usize>,
) {
    entries.sort_by_key(|(_, info)| info.order);

    let mut seen_locations = HashSet::new();
    for (cop_name, info) in entries {
        if seen_locations.insert((info.line, info.col)) {
            diagnostics.push(cop.diagnostic(
                source,
                info.line,
                info.col,
                format_message(&cop_name, max_range),
            ));
        }
    }
}

fn format_message(cop: &str, max_range: Option<usize>) -> String {
    // Determine if it's a department (no `/`) or a specific cop
    let kind = if cop.contains('/') {
        "cop"
    } else {
        "department"
    };
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

    let after_prefix = after_hash.strip_prefix("rubocop:")?.trim_start();

    // Extract action
    let action_end = after_prefix
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(after_prefix.len());
    let action = &after_prefix[..action_end];

    if !matches!(action, "disable" | "enable" | "todo") {
        return None;
    }

    // RuboCop parses only the leading valid cop list and ignores trailing freeform
    // text for this cop, even when another cop later flags the directive as malformed.
    let cops = parse_cop_list(&after_prefix[action_end..]);
    if cops.is_empty() {
        return None;
    }

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

fn parse_cop_list(raw: &str) -> Vec<String> {
    let mut remaining = raw;
    let mut cops = Vec::new();

    loop {
        remaining = remaining.trim_start();
        let Some((cop, rest)) = parse_cop_prefix(remaining) else {
            break;
        };

        cops.push(cop);
        remaining = rest.trim_start();

        let Some(rest_after_comma) = remaining.strip_prefix(',') else {
            break;
        };
        remaining = rest_after_comma;
    }

    cops
}

fn parse_cop_prefix(raw: &str) -> Option<(String, &str)> {
    let bytes = raw.as_bytes();
    let mut idx = 0usize;

    if !bytes.get(idx)?.is_ascii_alphabetic() {
        return None;
    }

    idx += 1;
    while idx < bytes.len() && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'_') {
        idx += 1;
    }

    while idx < bytes.len() && bytes[idx] == b'/' {
        let segment_start = idx + 1;
        let Some(next) = bytes.get(segment_start) else {
            break;
        };
        if !next.is_ascii_alphabetic() {
            break;
        }

        idx = segment_start + 1;
        while idx < bytes.len() && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'_') {
            idx += 1;
        }
    }

    Some((raw[..idx].to_string(), &raw[idx..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        MissingCopEnableDirective,
        "cops/lint/missing_cop_enable_directive",
        missing_enable_cop = "missing_enable_cop.rb",
        missing_enable_dept = "missing_enable_dept.rb",
        missing_enable_dept_trailing_slash = "missing_enable_dept_trailing_slash.rb",
        missing_enable_parameter_lists = "missing_enable_parameter_lists.rb",
        missing_enable_multi_metrics = "missing_enable_multi_metrics.rb",
        missing_enable_two = "missing_enable_two.rb",
        missing_enable_spaced = "missing_enable_spaced.rb",
    );
}
