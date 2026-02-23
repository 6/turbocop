use crate::cop::util::is_blank_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
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

/// Check if a line is a class/module opening or block opening that serves as
/// a boundary before an access modifier (no blank line required).
fn is_body_opening(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    if trimmed.is_empty() {
        return false;
    }
    // class/module definition
    if trimmed.starts_with(b"class ")
        || trimmed.starts_with(b"class\n")
        || trimmed == b"class"
        || trimmed.starts_with(b"module ")
        || trimmed.starts_with(b"module\n")
        || trimmed == b"module"
    {
        return true;
    }
    // Block opening: line ends with `do`, `do |...|`, or `{`
    // Strip trailing whitespace and carriage return
    let end_trimmed: Vec<u8> = trimmed
        .iter()
        .copied()
        .rev()
        .skip_while(|&b| b == b' ' || b == b'\t' || b == b'\r')
        .collect::<Vec<u8>>()
        .into_iter()
        .rev()
        .collect();
    if end_trimmed.ends_with(b"do") {
        // Make sure "do" is a keyword, not part of a word like "undo"
        let before_do = end_trimmed.len() - 2;
        if before_do == 0
            || !end_trimmed[before_do - 1].is_ascii_alphanumeric()
                && end_trimmed[before_do - 1] != b'_'
        {
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
    let trimmed: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    if trimmed.is_empty() {
        return false;
    }
    trimmed == b"end"
        || trimmed.starts_with(b"end ")
        || trimmed.starts_with(b"end\n")
        || trimmed.starts_with(b"end\r")
        || trimmed.starts_with(b"end#")
}

/// AST visitor that collects byte offsets of bare access modifier calls that are
/// direct children of class/module/singleton_class bodies (not block bodies).
struct AccessModifierCollector {
    /// Start byte offsets of access modifier calls in class/module bodies.
    offsets: Vec<usize>,
    /// Whether the current context is a class/module body.
    in_class_body: bool,
}

impl AccessModifierCollector {
    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        if !self.in_class_body {
            return;
        }
        if call.receiver().is_some() {
            return;
        }
        let method_name = call.name().as_slice();
        if !ACCESS_MODIFIERS.contains(&method_name) {
            return;
        }
        if call.arguments().is_some() {
            return;
        }
        if call.block().is_some() {
            return;
        }
        self.offsets.push(call.location().start_offset());
    }
}

impl<'pr> ruby_prism::Visit<'pr> for AccessModifierCollector {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let old = self.in_class_body;
        self.in_class_body = true;
        ruby_prism::visit_class_node(self, node);
        self.in_class_body = old;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let old = self.in_class_body;
        self.in_class_body = true;
        ruby_prism::visit_module_node(self, node);
        self.in_class_body = old;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let old = self.in_class_body;
        self.in_class_body = true;
        ruby_prism::visit_singleton_class_node(self, node);
        self.in_class_body = old;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let old = self.in_class_body;
        self.in_class_body = false;
        ruby_prism::visit_block_node(self, node);
        self.in_class_body = old;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let old = self.in_class_body;
        self.in_class_body = false;
        ruby_prism::visit_lambda_node(self, node);
        self.in_class_body = old;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_call(node);
        ruby_prism::visit_call_node(self, node);
    }
}

impl Cop for EmptyLinesAroundAccessModifier {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundAccessModifier"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "around");

        // Collect access modifier offsets that are in class/module bodies
        let mut collector = AccessModifierCollector {
            offsets: Vec::new(),
            in_class_body: false,
        };
        use ruby_prism::Visit;
        collector.visit(&parse_result.node());

        let lines: Vec<&[u8]> = source.lines().collect();

        for offset in collector.offsets {
            let (line, col) = source.offset_to_line_col(offset);

            // Determine the method name from the source at this offset
            let bytes = source.as_bytes();
            let method_name = ACCESS_MODIFIERS.iter().find(|&&m| {
                offset + m.len() <= bytes.len() && &bytes[offset..offset + m.len()] == m
            });
            let method_name = match method_name {
                Some(m) => *m,
                None => continue,
            };

            // Ensure the access modifier is the only thing on its line
            if line > 0 && line <= lines.len() {
                let current_line = lines[line - 1];
                let trimmed: Vec<u8> = current_line
                    .iter()
                    .copied()
                    .skip_while(|&b| b == b' ' || b == b'\t')
                    .collect();
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
                    continue;
                }
            }

            let modifier_str = std::str::from_utf8(method_name).unwrap_or("");

            // Find the previous non-comment line
            let has_blank_before = {
                let mut found_blank_or_boundary = true;
                let mut idx = line as isize - 2;
                while idx >= 0 {
                    let prev = lines[idx as usize];
                    if is_comment_line(prev) {
                        idx -= 1;
                        continue;
                    }
                    found_blank_or_boundary = is_blank_line(prev) || is_body_opening(prev);
                    break;
                }
                found_blank_or_boundary
            };

            // Check blank line after
            let has_blank_after = if line < lines.len() {
                let next = lines[line];
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
                                if let Some(off) = source.line_col_to_offset(line, 0) {
                                    corr.push(crate::correction::Correction {
                                        start: off,
                                        end: off,
                                        replacement: "\n".to_string(),
                                        cop_name: self.name(),
                                        cop_index: 0,
                                    });
                                    diag.corrected = true;
                                }
                            }
                            if !has_blank_after {
                                if let Some(off) = source.line_col_to_offset(line + 1, 0) {
                                    corr.push(crate::correction::Correction {
                                        start: off,
                                        end: off,
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
                            source,
                            line,
                            col,
                            format!("Keep a blank line before `{modifier_str}`."),
                        );
                        if let Some(ref mut corr) = corrections {
                            if let Some(off) = source.line_col_to_offset(line, 0) {
                                corr.push(crate::correction::Correction {
                                    start: off,
                                    end: off,
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
                _ => {}
            }
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
