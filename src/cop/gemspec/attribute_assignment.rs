use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AttributeAssignment;

impl Cop for AttributeAssignment {
    fn name(&self) -> &'static str {
        "Gemspec/AttributeAssignment"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Track attribute assignments: attr_name -> (line_num, style)
        // style is "direct" for spec.name = or "indexed" for spec.metadata["name"] =
        let mut seen: HashMap<String, (usize, &str)> = HashMap::new();

        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let line_num = line_idx + 1;

            // Check for duplicate attribute assignments
            if let Some((attr, _style)) = extract_attr_assignment(trimmed) {
                if let Some(&(first_line, _first_style)) = seen.get(&attr) {
                    let dot_pos = line_str.find('.').unwrap_or(0);
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        dot_pos + 1,
                        format!("Attribute `{attr}` is already set on line {first_line}."),
                    ));
                } else {
                    seen.insert(attr, (line_num, _style));
                }
            }
        }
    }
}

/// Extract attribute name and assignment style from a line.
/// Returns (attr_name, "direct"|"indexed") or None.
fn extract_attr_assignment(trimmed: &str) -> Option<(String, &'static str)> {
    // Look for a dot after a variable name
    let dot_pos = trimmed.find('.')?;
    let after_dot = &trimmed[dot_pos + 1..];

    // Check for metadata["key"] = pattern
    if after_dot.starts_with("metadata[") || after_dot.starts_with("metadata [") {
        let bracket_start = after_dot.find('[')?;
        let bracket_end = after_dot.find(']')?;
        let key_part = &after_dot[bracket_start + 1..bracket_end];
        // Strip quotes
        let key = key_part.trim_matches(|c| c == '\'' || c == '"');
        let rest = after_dot[bracket_end + 1..].trim_start();
        if rest.starts_with('=') && !rest.starts_with("==") {
            return Some((format!("metadata[{key}]"), "indexed"));
        }
        return None;
    }

    // Check for direct assignment: attr_name = ...
    let attr_end = after_dot
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(after_dot.len());
    if attr_end == 0 {
        return None;
    }
    let attr = &after_dot[..attr_end];
    let rest = after_dot[attr_end..].trim_start();

    if rest.starts_with('=') && !rest.starts_with("==") {
        Some((attr.to_string(), "direct"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AttributeAssignment, "cops/gemspec/attribute_assignment");
}
