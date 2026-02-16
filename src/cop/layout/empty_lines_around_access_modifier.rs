use crate::cop::util::is_blank_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundAccessModifier;

const ACCESS_MODIFIERS: &[&[u8]] = &[b"private", b"protected", b"public", b"module_function"];

/// Check if a line is a comment (first non-whitespace character is `#`).
fn is_comment_line(line: &[u8]) -> bool {
    for &b in line {
        if b == b' ' || b == b'\t' {
            continue;
        }
        return b == b'#';
    }
    false
}

/// Check if a line is a class/module opening (serves as boundary before access modifier).
fn is_class_or_module_opening(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line.iter().copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.starts_with(b"class ") || trimmed.starts_with(b"class\n") || trimmed == b"class"
        || trimmed.starts_with(b"module ") || trimmed.starts_with(b"module\n") || trimmed == b"module"
}

/// Check if a line is just `end` (possibly with trailing whitespace/comment).
/// Used to detect body-end boundary after access modifier.
fn is_end_line(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line.iter().copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    if trimmed.is_empty() {
        return false;
    }
    trimmed == b"end" || trimmed.starts_with(b"end ")
        || trimmed.starts_with(b"end\n") || trimmed.starts_with(b"end\r")
        || trimmed.starts_with(b"end#")
}

impl Cop for EmptyLinesAroundAccessModifier {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundAccessModifier"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "around");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Check if it's a bare access modifier (no receiver, no args, no block)
        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if !ACCESS_MODIFIERS.iter().any(|&m| m == method_name) {
            return Vec::new();
        }

        // Must have no arguments to be an access modifier (not `private :foo`)
        if call.arguments().is_some() {
            return Vec::new();
        }

        // Skip if it has a block (e.g. `private do ... end`)
        if call.block().is_some() {
            return Vec::new();
        }

        let loc = call.location();
        let (line, col) = source.offset_to_line_col(loc.start_offset());
        let lines: Vec<&[u8]> = source.lines().collect();

        let modifier_str = std::str::from_utf8(method_name).unwrap_or("");

        // Find the previous non-comment line (RuboCop skips comments when checking blank before)
        let has_blank_before = {
            let mut found_blank_or_boundary = true; // default true (first line in body)
            let mut idx = line as isize - 2; // line is 1-based, lines[] is 0-based
            // Skip comment lines
            while idx >= 0 {
                let prev = lines[idx as usize];
                if is_comment_line(prev) {
                    idx -= 1;
                    continue;
                }
                // Found a non-comment line: check if it's blank or a class/module opening
                found_blank_or_boundary = is_blank_line(prev) || is_class_or_module_opening(prev);
                break;
            }
            found_blank_or_boundary
        };

        // Check blank line after (or body end boundary — `end` of class/module)
        let has_blank_after = if line < lines.len() {
            let next = lines[line]; // line is 1-based, so lines[line] is the next line
            is_blank_line(next) || is_end_line(next)
        } else {
            true
        };

        match enforced_style {
            "around" => {
                if !has_blank_before && !has_blank_after {
                    vec![self.diagnostic(
                        source,
                        line,
                        col,
                        format!("Keep a blank line before and after `{modifier_str}`."),
                    )]
                } else if !has_blank_before {
                    vec![self.diagnostic(
                        source,
                        line,
                        col,
                        format!("Keep a blank line before and after `{modifier_str}`."),
                    )]
                } else if !has_blank_after {
                    vec![self.diagnostic(
                        source,
                        line,
                        col,
                        format!("Keep a blank line after `{modifier_str}`."),
                    )]
                } else {
                    Vec::new()
                }
            }
            "only_before" => {
                if !has_blank_before {
                    vec![self.diagnostic(
                        source,
                        line,
                        col,
                        format!("Keep a blank line before `{modifier_str}`."),
                    )]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAroundAccessModifier,
        "cops/layout/empty_lines_around_access_modifier"
    );
}
