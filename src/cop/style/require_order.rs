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

        // Groups are separated by blank lines or non-require lines.
        // `require` and `require_relative` are separate groups even if adjacent.
        let mut groups: Vec<Vec<(usize, String, &str)>> = Vec::new(); // (line, path, kind)
        let mut current_group: Vec<(usize, String, &str)> = Vec::new();
        let mut current_kind: &str = "";

        for (i, line) in lines.iter().enumerate() {
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
            for window in group.windows(2) {
                let (_, ref prev_path, _) = window[0];
                let (line_num, ref curr_path, _) = window[1];
                if curr_path < prev_path {
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        0,
                        format!("Sort `{}` in alphabetical order.", kind),
                    ));
                }
            }
        }

        diagnostics
    }
}

fn extract_require_path_and_kind(line: &str) -> Option<(String, &'static str)> {
    let line = line.trim();
    let (rest, kind) = if let Some(r) = line.strip_prefix("require_relative ") {
        (r, "require_relative")
    } else if let Some(r) = line.strip_prefix("require ") {
        (r, "require")
    } else {
        return None;
    };

    let rest = rest.trim();
    // Extract string argument — handle `require 'x' if cond` (modifier conditional)
    let quote = rest.as_bytes().first()?;
    if *quote != b'\'' && *quote != b'"' {
        return None;
    }
    // Find the closing quote
    let end_pos = rest[1..].find(*quote as char).map(|p| p + 1)?;
    let inner = &rest[1..end_pos];
    Some((inner.to_string(), kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RequireOrder, "cops/style/require_order");
}
