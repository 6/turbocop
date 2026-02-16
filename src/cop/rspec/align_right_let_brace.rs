use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AlignRightLetBrace;

impl Cop for AlignRightLetBrace {
    fn name(&self) -> &'static str {
        "RSpec/AlignRightLetBrace"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        // Step 1: Collect all single-line let positions (1-indexed line, close_brace_col)
        let lets: Vec<(usize, usize)> = lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| {
                single_line_let_close_brace_col(line).map(|col| (i + 1, col))
            })
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
                            "Align right let brace.".to_string(),
                        ));
                    }
                }
            }
        }

        diagnostics
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

    let mut keys: Vec<bool> = Vec::with_capacity(lets.len());
    let mut last_line: Option<usize> = None;

    for &(line, _) in lets {
        let is_adjacent = last_line.is_none() || last_line.map_or(false, |prev| prev + 1 == line);
        if is_adjacent {
            last_line = Some(line);
        } else {
            last_line = None;
        }
        keys.push(last_line.is_none());
    }

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

fn single_line_let_close_brace_col(line: &[u8]) -> Option<usize> {
    let s = std::str::from_utf8(line).ok()?;
    let trimmed = s.trim_start();
    if !trimmed.starts_with("let(") && !trimmed.starts_with("let!(") {
        return None;
    }

    // Must have opening and closing brace on same line
    let open_brace = s.find('{')?;
    let close_brace = s.rfind('}')?;

    if close_brace <= open_brace {
        return None;
    }

    Some(close_brace)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AlignRightLetBrace, "cops/rspec/align_right_let_brace");
}
