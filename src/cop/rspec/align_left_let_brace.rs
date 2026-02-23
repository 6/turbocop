use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AlignLeftLetBrace;

impl Cop for AlignLeftLetBrace {
    fn name(&self) -> &'static str {
        "RSpec/AlignLeftLetBrace"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let lines: Vec<&[u8]> = source.lines().collect();

        // Step 1: Collect all single-line let positions (1-indexed line, brace_col)
        let lets: Vec<(usize, usize)> = lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| single_line_let_brace_col(line).map(|col| (i + 1, col)))
            .collect();

        // Step 2: Group by strictly consecutive line numbers, replicating RuboCop's
        // chunking behavior where after a gap the first let is isolated.
        let groups = chunk_adjacent_lets(&lets);

        // Step 3: Check alignment within each group
        for group in &groups {
            if group.len() >= 2 {
                let max_col = group.iter().map(|(_, c)| *c).max().unwrap_or(0);
                for &(line_num, brace_col) in group {
                    if brace_col != max_col {
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            brace_col,
                            "Align left let brace.".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

/// Replicate RuboCop's `adjacent_let_chunks` grouping: walk sorted single-line
/// lets and chunk by consecutive line numbers. After a gap, the first let is
/// isolated into its own singleton group (matching the Ruby `Enumerable#chunk`
/// behavior with the nil-reset pattern used in `align_let_brace.rb`).
fn chunk_adjacent_lets(lets: &[(usize, usize)]) -> Vec<Vec<(usize, usize)>> {
    if lets.is_empty() {
        return Vec::new();
    }

    // Compute the chunk key for each let, mirroring RuboCop's logic:
    //   last_line = nil
    //   chunk { |node| line = node.line; last_line = (line if last_line.nil? || last_line+1 == line); last_line.nil? }
    let mut keys: Vec<bool> = Vec::with_capacity(lets.len());
    let mut last_line: Option<usize> = None;

    for &(line, _) in lets {
        let is_adjacent = last_line.is_none() || last_line.is_some_and(|prev| prev + 1 == line);
        if is_adjacent {
            last_line = Some(line);
        } else {
            last_line = None;
        }
        keys.push(last_line.is_none());
    }

    // Group consecutive elements with the same key (Ruby's Enumerable#chunk)
    let mut groups: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut prev_key: Option<bool> = None;

    for (i, &key) in keys.iter().enumerate() {
        if prev_key == Some(key) {
            groups.last_mut().unwrap().push(lets[i]);
        } else {
            groups.push(vec![lets[i]]);
            prev_key = Some(key);
        }
    }

    groups
}

fn single_line_let_brace_col(line: &[u8]) -> Option<usize> {
    let s = std::str::from_utf8(line).ok()?;
    let trimmed = s.trim_start();
    if !trimmed.starts_with("let(") && !trimmed.starts_with("let!(") {
        return None;
    }

    // Find `) {` pattern - single-line let with braces
    let paren_close = s.find(')')?;
    let after_paren = &s[paren_close + 1..];
    let trimmed_after = after_paren.trim_start();

    if !trimmed_after.starts_with('{') {
        return None;
    }

    // Must also have closing brace on same line
    let brace_open = paren_close + 1 + (after_paren.len() - trimmed_after.len());
    if !s[brace_open..].contains('}') {
        return None;
    }

    Some(brace_open)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AlignLeftLetBrace, "cops/rspec/align_left_let_brace");
}
