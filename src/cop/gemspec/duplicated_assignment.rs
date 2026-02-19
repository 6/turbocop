use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DuplicatedAssignment;

impl Cop for DuplicatedAssignment {
    fn name(&self) -> &'static str {
        "Gemspec/DuplicatedAssignment"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>) {
        // Track: attribute_name -> first occurrence line
        let mut seen: HashMap<String, usize> = HashMap::new();

        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            // Skip comments and blank lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Match patterns like `spec.name = ...` or `s.version = ...`
            // but not `spec.requirements <<` (append) or method calls without `=`
            if let Some(attr) = extract_assignment_attr(trimmed) {
                let line_num = line_idx + 1;
                if seen.contains_key(&attr) {
                    // Find the position of the attribute in the original line
                    let dot_pos = line_str.find('.').unwrap_or(0);
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        dot_pos + 1, // after the dot
                        format!("Attribute `{attr}` is already set on line {}.", seen[&attr]),
                    ));
                } else {
                    seen.insert(attr, line_num);
                }
            }
        }
    }
}

/// Extract the attribute name from a line like `spec.name = 'foo'`.
/// Returns None for non-assignment lines or append operations (<<).
fn extract_assignment_attr(trimmed: &str) -> Option<String> {
    // Look for pattern: <identifier>.<attribute> = ...
    // Must have a dot, then an identifier, then ` = ` or `= `
    let dot_pos = trimmed.find('.')?;
    let after_dot = &trimmed[dot_pos + 1..];

    // Extract attribute name (alphanumeric + underscore)
    let attr_end = after_dot
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(after_dot.len());
    if attr_end == 0 {
        return None;
    }
    let attr = &after_dot[..attr_end];
    let rest = after_dot[attr_end..].trim_start();

    // Must be followed by `=` but not `==` or `<<`
    if rest.starts_with("= ") || rest.starts_with("=\n") || rest == "=" {
        // But not `==`
        if rest.starts_with("==") {
            return None;
        }
        Some(attr.to_string())
    } else if rest.starts_with('=') && rest.len() > 1 && rest.as_bytes()[1] != b'=' {
        Some(attr.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicatedAssignment, "cops/gemspec/duplicated_assignment");
}
