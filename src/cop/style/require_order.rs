use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/RequireOrder: Sort `require` and `require_relative` in alphabetical order.
///
/// Investigation findings (FP=117, FN=373):
/// - FP root cause: nitrocop was flagging `require` with string interpolation (e.g.,
///   `require "#{base}/foo"`). RuboCop only checks `str_type?` arguments, skipping `dstr`
///   (interpolated strings). Fixed by rejecting paths containing `#{`.
/// - FN root cause: nitrocop treated comment lines (including `# require 'foo'`) as group
///   separators. RuboCop's AST-based approach treats comments as transparent since they
///   aren't sibling nodes; only blank lines (`\n\n`) break groups via `in_same_section?`.
///   Fixed by making comment lines transparent in group formation.
/// - Remaining: interpolated-string requires now act as group separators (matching RuboCop),
///   since they fail `str_type?` and break the sibling walk.
pub struct RequireOrder;

impl Cop for RequireOrder {
    fn name(&self) -> &'static str {
        "Style/RequireOrder"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let lines: Vec<&[u8]> = source.lines().collect();

        // Compute byte offsets where each line starts
        let mut line_offsets = Vec::with_capacity(lines.len());
        let mut offset = 0usize;
        for line in &lines {
            line_offsets.push(offset);
            offset += line.len() + 1; // +1 for the newline
        }

        // Groups are separated by blank lines or non-require/non-comment lines.
        // `require` and `require_relative` are separate groups even if adjacent.
        // Comment lines are transparent — they don't break groups (matching RuboCop's
        // AST-based approach where comments aren't sibling nodes).
        let mut groups: Vec<Vec<(usize, String, &str)>> = Vec::new(); // (line, path, kind)
        let mut current_group: Vec<(usize, String, &str)> = Vec::new();
        let mut current_kind: &str = "";

        for (i, line) in lines.iter().enumerate() {
            // Skip lines inside heredocs
            if i < line_offsets.len() && code_map.is_heredoc(line_offsets[i]) {
                if current_group.len() > 1 {
                    groups.push(std::mem::take(&mut current_group));
                } else {
                    current_group.clear();
                }
                current_kind = "";
                continue;
            }

            let trimmed = std::str::from_utf8(line).unwrap_or("").trim();
            if let Some((path, kind)) = extract_require_path_and_kind(trimmed) {
                // If the kind changed (require vs require_relative), start a new group
                if !current_group.is_empty() && kind != current_kind {
                    if current_group.len() > 1 {
                        groups.push(std::mem::take(&mut current_group));
                    } else {
                        current_group.clear();
                    }
                }
                current_kind = kind;
                current_group.push((i + 1, path, kind));
            } else if is_comment_line(trimmed) {
                // Comment lines are transparent — don't break groups
            } else {
                if current_group.len() > 1 {
                    groups.push(std::mem::take(&mut current_group));
                } else {
                    current_group.clear();
                }
                current_kind = "";
            }
        }
        if current_group.len() > 1 {
            groups.push(current_group);
        }

        for group in &groups {
            let kind = group[0].2;
            // Track the maximum path seen so far. An entry is out of order
            // if its path is less than ANY previous path in the group,
            // which is equivalent to being less than the running maximum.
            let mut max_path: &str = &group[0].1;
            for &(line_num, ref curr_path, _) in &group[1..] {
                if curr_path.as_str() < max_path {
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        0,
                        format!("Sort `{}` in alphabetical order.", kind),
                    ));
                } else {
                    max_path = curr_path;
                }
            }
        }
    }
}

fn extract_require_path_and_kind(line: &str) -> Option<(String, &'static str)> {
    let line = line.trim();
    // Match `require_relative` before `require` to avoid prefix collision
    let (rest, kind) = if let Some(r) = line.strip_prefix("require_relative") {
        if r.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
            return None;
        }
        (r, "require_relative")
    } else if let Some(r) = line.strip_prefix("require") {
        if r.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
            return None;
        }
        (r, "require")
    } else {
        return None;
    };

    // Handle both `require 'x'` and `require('x')` / `require_relative("x")` syntax
    let rest = rest.trim_start();
    let rest = rest
        .strip_prefix('(')
        .map(|r| r.trim_start())
        .unwrap_or(rest);

    // Extract string argument — handle `require 'x' if cond` (modifier conditional)
    let quote = rest.as_bytes().first()?;
    if *quote != b'\'' && *quote != b'"' {
        return None;
    }
    // Find the closing quote
    let end_pos = rest[1..].find(*quote as char).map(|p| p + 1)?;
    let inner = &rest[1..end_pos];
    // Skip strings with interpolation — RuboCop only checks str_type? (not dstr)
    if inner.contains("#{") {
        return None;
    }
    Some((inner.to_string(), kind))
}

/// Returns true if the line is a comment (starts with `#`).
fn is_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with('#')
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RequireOrder, "cops/style/require_order");
}
