use crate::cop::util::is_blank_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

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

/// Check if a line is a class/module opening or block opening that serves as
/// a boundary before an access modifier (no blank line required).
fn is_body_opening(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line.iter().copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    if trimmed.is_empty() {
        return false;
    }
    // class/module definition
    if trimmed.starts_with(b"class ") || trimmed.starts_with(b"class\n") || trimmed == b"class"
        || trimmed.starts_with(b"module ") || trimmed.starts_with(b"module\n") || trimmed == b"module"
    {
        return true;
    }
    // Block opening: line ends with `do`, `do |...|`, or `{`
    // Strip trailing whitespace and carriage return
    let end_trimmed: Vec<u8> = trimmed.iter().copied()
        .rev()
        .skip_while(|&b| b == b' ' || b == b'\t' || b == b'\r')
        .collect::<Vec<u8>>()
        .into_iter()
        .rev()
        .collect();
    if end_trimmed.ends_with(b"do") {
        // Make sure "do" is a keyword, not part of a word like "undo"
        let before_do = end_trimmed.len() - 2;
        if before_do == 0 || !end_trimmed[before_do - 1].is_ascii_alphanumeric() && end_trimmed[before_do - 1] != b'_' {
            return true;
        }
    }
    // Block opening with `do |...|`
    if end_trimmed.ends_with(b"|") {
        // Look for `do ` or `do|` pattern somewhere in the line
        if end_trimmed.windows(3).any(|w| w == b"do " || w == b"do|") {
            return true;
        }
    }
    if end_trimmed.ends_with(b"{") {
        return true;
    }
    false
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

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "around");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Check if it's a bare access modifier (no receiver, no args, no block)
        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if !ACCESS_MODIFIERS.iter().any(|&m| m == method_name) {
            return;
        }

        // Must have no arguments to be an access modifier (not `private :foo`)
        if call.arguments().is_some() {
            return;
        }

        // Skip if it has a block (e.g. `private do ... end`)
        if call.block().is_some() {
            return;
        }

        let loc = call.location();
        let (line, col) = source.offset_to_line_col(loc.start_offset());
        let lines: Vec<&[u8]> = source.lines().collect();

        // Ensure the access modifier is the only thing on its line (after trimming
        // whitespace). This filters out false positives like `private: private` in
        // hash literals or `!private` in conditionals.
        if line > 0 && line <= lines.len() {
            let current_line = lines[line - 1];
            let trimmed: Vec<u8> = current_line
                .iter()
                .copied()
                .skip_while(|&b| b == b' ' || b == b'\t')
                .collect();
            // The trimmed line should be exactly the modifier keyword (possibly with trailing whitespace/CR)
            let end_trimmed: Vec<u8> = trimmed
                .iter()
                .copied()
                .rev()
                .skip_while(|&b| b == b' ' || b == b'\t' || b == b'\r' || b == b'\n')
                .collect::<Vec<u8>>()
                .into_iter()
                .rev()
                .collect();
            if end_trimmed != method_name {
                return;
            }
        }

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
                found_blank_or_boundary = is_blank_line(prev) || is_body_opening(prev);
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
                if !has_blank_before || !has_blank_after {
                    let msg = if !has_blank_after && has_blank_before {
                        format!("Keep a blank line after `{modifier_str}`.")
                    } else {
                        format!("Keep a blank line before and after `{modifier_str}`.")
                    };
                    let mut diag = self.diagnostic(source, line, col, msg);
                    if let Some(ref mut corr) = corrections {
                        if !has_blank_before {
                            if let Some(offset) = source.line_col_to_offset(line, 0) {
                                corr.push(crate::correction::Correction {
                                    start: offset,
                                    end: offset,
                                    replacement: "\n".to_string(),
                                    cop_name: self.name(),
                                    cop_index: 0,
                                });
                                diag.corrected = true;
                            }
                        }
                        if !has_blank_after {
                            if let Some(offset) = source.line_col_to_offset(line + 1, 0) {
                                corr.push(crate::correction::Correction {
                                    start: offset,
                                    end: offset,
                                    replacement: "\n".to_string(),
                                    cop_name: self.name(),
                                    cop_index: 0,
                                });
                                diag.corrected = true;
                            }
                        }
                    }
                    diagnostics.push(diag);
                }
            }
            "only_before" => {
                if !has_blank_before {
                    let mut diag = self.diagnostic(
                        source, line, col,
                        format!("Keep a blank line before `{modifier_str}`."),
                    );
                    if let Some(ref mut corr) = corrections {
                        if let Some(offset) = source.line_col_to_offset(line, 0) {
                            corr.push(crate::correction::Correction {
                                start: offset,
                                end: offset,
                                replacement: "\n".to_string(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                    }
                    diagnostics.push(diag);
                }
            }
            _ => {},
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
    crate::cop_autocorrect_fixture_tests!(
        EmptyLinesAroundAccessModifier,
        "cops/layout/empty_lines_around_access_modifier"
    );
}
