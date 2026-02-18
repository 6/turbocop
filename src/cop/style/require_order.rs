use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RequireOrder;

impl Cop for RequireOrder {
    fn name(&self) -> &'static str {
        "Style/RequireOrder"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines: Vec<&[u8]> = source.lines().collect();

        let mut groups: Vec<Vec<(usize, String)>> = Vec::new();
        let mut current_group: Vec<(usize, String)> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = std::str::from_utf8(line).unwrap_or("").trim();
            if let Some(path) = extract_require_path(trimmed) {
                current_group.push((i + 1, path));
            } else {
                if current_group.len() > 1 {
                    groups.push(std::mem::take(&mut current_group));
                } else {
                    current_group.clear();
                }
            }
        }
        if current_group.len() > 1 {
            groups.push(current_group);
        }

        for group in &groups {
            for window in group.windows(2) {
                let (_, ref prev_path) = window[0];
                let (line_num, ref curr_path) = window[1];
                if curr_path < prev_path {
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        0,
                        format!(
                            "Sort `require` and `require_relative` in alphabetical order.",
                        ),
                    ));
                }
            }
        }

        diagnostics
    }
}

fn extract_require_path(line: &str) -> Option<String> {
    let line = line.trim();
    let rest = if let Some(r) = line.strip_prefix("require ") {
        r
    } else if let Some(r) = line.strip_prefix("require_relative ") {
        r
    } else {
        return None;
    };

    let rest = rest.trim();
    // Extract string argument
    if (rest.starts_with('\'') && rest.ends_with('\''))
        || (rest.starts_with('"') && rest.ends_with('"'))
    {
        let inner = &rest[1..rest.len() - 1];
        return Some(inner.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RequireOrder, "cops/style/require_order");
}
