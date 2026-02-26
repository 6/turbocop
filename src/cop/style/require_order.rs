use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RequireOrder;

impl Cop for RequireOrder {
    fn name(&self) -> &'static str {
        "Style/RequireOrder"
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

        // Groups are separated by blank lines or non-require lines.
        // `require` and `require_relative` are separate groups even if adjacent.
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
