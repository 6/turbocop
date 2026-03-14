use crate::cop::util::is_blank_or_whitespace_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Layout/EmptyLinesAroundAccessModifier
///
/// Investigation findings (2026-03-11):
///
/// FP root causes:
/// 1. Visitor did not exclude `def`/`defs` bodies — any `private` call inside a
///    method body in a class was incorrectly collected as an access modifier.
///    Fix: added `visit_def_node` to set `in_class_body = false`.
/// 2. Multiline class/module definitions (`class Foo <\n  Bar`) were not recognized
///    as body openings. The text-based `is_body_opening` only checked if the previous
///    line started with `class`/`module`, missing the continuation line.
///    Fix: store class/module/block opening lines from the AST in the collector, and
///    use those for boundary detection instead of text matching.
/// 3. Whitespace-only "blank" lines (e.g., lines with trailing spaces/tabs) were
///    not recognized as blank by `is_blank_line`. Repos like coderay and redcar use
///    trailing whitespace on otherwise empty lines.
///    Fix: switched to `is_blank_or_whitespace_line` (2026-03-14).
///
/// FN root causes:
/// 1. Access modifiers with trailing comments (`private # comment`) were rejected by
///    the line-content check which required `end_trimmed == method_name`.
///    Fix: allow trailing `# comment` after the modifier.
/// 2. Access modifiers inside blocks (`included do ... end`) were excluded by the
///    visitor (pushed as non-class scope), but RuboCop treats them as valid scopes.
///    Fix: treat blocks in class/module scope (or top-level) as class-like scopes,
///    while blocks inside method bodies remain excluded (2026-03-14).
/// 3. `only_before` style: missing "Remove a blank line after" offense for
///    `private`/`protected`. Not yet fixed.
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

/// Check if a line contains only the access modifier keyword (possibly with a
/// trailing comment). Returns true for `private`, `private # comment`, etc.
fn is_bare_modifier_line(line: &[u8], method_name: &[u8]) -> bool {
    let trimmed: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    // Strip trailing whitespace/newline
    let end_trimmed: Vec<u8> = trimmed
        .iter()
        .copied()
        .rev()
        .skip_while(|&b| b == b' ' || b == b'\t' || b == b'\r' || b == b'\n')
        .collect::<Vec<u8>>()
        .into_iter()
        .rev()
        .collect();
    // Exact match: just the modifier keyword
    if end_trimmed == method_name {
        return true;
    }
    // Modifier followed by whitespace then comment: `private # comment`
    if end_trimmed.starts_with(method_name) {
        let rest = &end_trimmed[method_name.len()..];
        // After the modifier, skip whitespace then expect `#`
        let after_ws: Vec<u8> = rest
            .iter()
            .copied()
            .skip_while(|&b| b == b' ' || b == b'\t')
            .collect();
        if after_ws.starts_with(b"#") {
            return true;
        }
    }
    false
}

/// Collected access modifier with context about its enclosing scope.
struct ModifierInfo {
    /// Byte offset of the access modifier call.
    offset: usize,
    /// The 1-based line number of the body opening of the enclosing class/module/block.
    /// For `class Foo < Bar`, this is Bar's line. For `class Foo`, this is the class line.
    /// For blocks, this is the block opening line.
    body_opening_line: usize,
    /// The 1-based line number of the `end` closing the enclosing class/module/block.
    body_closing_line: usize,
}

/// AST visitor that collects byte offsets of bare access modifier calls that are
/// direct children of class/module/singleton_class bodies (not method or lambda bodies).
struct AccessModifierCollector {
    /// Collected access modifier info.
    modifiers: Vec<ModifierInfo>,
    /// Stack of (in_class_body, body_opening_line, body_closing_line) for scope tracking.
    scope_stack: Vec<(bool, usize, usize)>,
}

impl AccessModifierCollector {
    fn in_class_body(&self) -> bool {
        self.scope_stack
            .last()
            .map(|(in_body, _, _)| *in_body)
            .unwrap_or(false)
    }

    fn current_scope(&self) -> (usize, usize) {
        self.scope_stack
            .last()
            .map(|(_, opening, closing)| (*opening, *closing))
            .unwrap_or((0, 0))
    }

    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        if !self.in_class_body() {
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
        let (body_opening_line, body_closing_line) = self.current_scope();
        self.modifiers.push(ModifierInfo {
            offset: call.location().start_offset(),
            body_opening_line,
            body_closing_line,
        });
    }

    fn push_class_scope(&mut self, body_opening_line: usize, body_closing_line: usize) {
        self.scope_stack
            .push((true, body_opening_line, body_closing_line));
    }

    fn push_non_class_scope(&mut self) {
        self.scope_stack.push((false, 0, 0));
    }

    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }
}

impl<'pr> ruby_prism::Visit<'pr> for AccessModifierCollector {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // For multiline class definitions like `class Foo <\n  Bar`,
        // the body opening line is the parent class's line (where Bar is).
        // For simple `class Foo`, it's the class keyword line.
        let opening_line = if let Some(superclass) = node.superclass() {
            superclass.location().start_offset()
        } else {
            node.location().start_offset()
        };
        let closing_line = node.location().end_offset();
        self.push_class_scope(opening_line, closing_line);
        ruby_prism::visit_class_node(self, node);
        self.pop_scope();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let opening = node.location().start_offset();
        let closing = node.location().end_offset();
        self.push_class_scope(opening, closing);
        ruby_prism::visit_module_node(self, node);
        self.pop_scope();
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        // For `class << self`, the expression is `self` — use its line as opening.
        let opening = node.expression().location().start_offset();
        let closing = node.location().end_offset();
        self.push_class_scope(opening, closing);
        ruby_prism::visit_singleton_class_node(self, node);
        self.pop_scope();
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        // Method bodies are not macro scopes — exclude them.
        self.push_non_class_scope();
        ruby_prism::visit_def_node(self, node);
        self.pop_scope();
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // RuboCop treats block bodies as valid scopes for access modifiers
        // (e.g., `included do`, `ActiveSupport.on_load do`, etc.), but only
        // when the block is in a class/module scope (not inside a method body).
        if self.in_class_body() || self.scope_stack.is_empty() {
            let opening = node.location().start_offset();
            let closing = node.location().end_offset();
            self.push_class_scope(opening, closing);
            ruby_prism::visit_block_node(self, node);
            self.pop_scope();
        } else {
            self.push_non_class_scope();
            ruby_prism::visit_block_node(self, node);
            self.pop_scope();
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.push_non_class_scope();
        ruby_prism::visit_lambda_node(self, node);
        self.pop_scope();
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
            modifiers: Vec::new(),
            scope_stack: Vec::new(),
        };
        use ruby_prism::Visit;
        collector.visit(&parse_result.node());

        let lines: Vec<&[u8]> = source.lines().collect();

        for modifier in &collector.modifiers {
            let (line, col) = source.offset_to_line_col(modifier.offset);

            // Determine the method name from the source at this offset
            let bytes = source.as_bytes();
            let method_name = ACCESS_MODIFIERS.iter().find(|&&m| {
                modifier.offset + m.len() <= bytes.len()
                    && &bytes[modifier.offset..modifier.offset + m.len()] == m
            });
            let method_name = match method_name {
                Some(m) => *m,
                None => continue,
            };

            // Ensure the access modifier is the only thing on its line (optionally with comment)
            if line > 0 && line <= lines.len() {
                let current_line = lines[line - 1];
                if !is_bare_modifier_line(current_line, method_name) {
                    continue;
                }
            }

            let modifier_str = std::str::from_utf8(method_name).unwrap_or("");

            // Convert body opening/closing offsets to 1-based line numbers
            let (body_opening_line, _) = source.offset_to_line_col(modifier.body_opening_line);
            let body_closing_offset = modifier.body_closing_line;
            // The closing offset points to the end of `end`, so the `end` keyword is on
            // the line containing that offset. We want the line before that.
            let body_closing_line = if body_closing_offset > 0 {
                let (cl, _) = source.offset_to_line_col(body_closing_offset - 1);
                cl
            } else {
                0
            };

            // Check if we're at a class/module body opening (line right after the opening)
            let is_at_body_opening = line == body_opening_line + 1;

            // Check if we're at a body end (line right before the closing `end`)
            let is_at_body_end = line == body_closing_line - 1;

            // Find the previous non-comment line
            let has_blank_before = {
                if is_at_body_opening {
                    true
                } else {
                    let mut found_blank_or_boundary = true;
                    let mut idx = line as isize - 2;
                    while idx >= 0 {
                        let prev = lines[idx as usize];
                        if is_comment_line(prev) {
                            idx -= 1;
                            continue;
                        }
                        found_blank_or_boundary =
                            is_blank_or_whitespace_line(prev) || is_body_opening(prev);
                        break;
                    }
                    found_blank_or_boundary
                }
            };

            // Check blank line after
            let has_blank_after = if is_at_body_end {
                true
            } else if line < lines.len() {
                let next = lines[line];
                is_blank_or_whitespace_line(next)
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
