use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct Semicolon;

impl Cop for Semicolon {
    fn name(&self) -> &'static str {
        "Style/Semicolon"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();
        if !bytes.contains(&b';') {
            return;
        }

        let allow_separator = config.get_bool("AllowAsExpressionSeparator", false);

        // Phase 1: Walk the AST to find lines where a StatementsNode has 2+ children
        // sharing the same last_line. These lines have expression separator semicolons.
        // RuboCop's on_begin fires for these and flags ALL semicolons on such lines.
        let expr_sep_lines = if !allow_separator {
            let mut visitor = ExprSeparatorVisitor {
                source,
                lines: HashSet::new(),
            };
            visitor.visit(&parse_result.node());
            visitor.lines
        } else {
            HashSet::new()
        };

        // Phase 2: Scan for code semicolons and classify each.
        for (i, &byte) in bytes.iter().enumerate() {
            if byte != b';' || !code_map.is_code(i) {
                continue;
            }

            let (line, column) = source.offset_to_line_col(i);

            // Check if trailing: no non-whitespace content after the semicolon on this line,
            // or only a comment follows. Uses raw bytes (not code_map) because string
            // literals after a semicolon count as meaningful content.
            if is_trailing_semicolon(bytes, i) {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use semicolons to terminate expressions.".to_string(),
                ));
                continue;
            }

            // Check if leading: nothing meaningful before the semicolon on this line.
            if is_leading_semicolon(bytes, i) {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use semicolons to terminate expressions.".to_string(),
                ));
                continue;
            }

            // Mid-line semicolon: only flag if on an expression separator line
            // (a line where a StatementsNode has 2+ children with the same last_line).
            if expr_sep_lines.contains(&line) {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not use semicolons to terminate expressions.".to_string(),
                ));
            }
        }
    }
}

/// Check if a semicolon at byte position `pos` is trailing:
/// nothing non-whitespace follows it on the same line (or only a comment follows).
fn is_trailing_semicolon(bytes: &[u8], pos: usize) -> bool {
    for &ch in &bytes[pos + 1..] {
        if ch == b'\n' || ch == b'\r' {
            return true;
        }
        if ch == b' ' || ch == b'\t' {
            continue;
        }
        if ch == b'#' {
            // Rest is a comment
            return true;
        }
        // Any non-whitespace, non-comment character means it's not trailing
        return false;
    }
    // Reached end of file without newline
    true
}

/// Check if a semicolon at byte position `pos` is leading:
/// nothing meaningful before it on this line (only whitespace).
fn is_leading_semicolon(bytes: &[u8], pos: usize) -> bool {
    if pos == 0 {
        return true;
    }
    for &ch in bytes[..pos].iter().rev() {
        if ch == b'\n' || ch == b'\r' {
            return true;
        }
        if ch == b' ' || ch == b'\t' {
            continue;
        }
        return false;
    }
    // Reached start of file
    true
}

/// AST visitor that collects line numbers where a StatementsNode has 2+ children
/// sharing the same last_line (expression separator lines).
struct ExprSeparatorVisitor<'a> {
    source: &'a SourceFile,
    lines: HashSet<usize>,
}

impl<'pr> Visit<'pr> for ExprSeparatorVisitor<'_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let body: Vec<ruby_prism::Node<'pr>> = node.body().iter().collect();
        if body.len() >= 2 {
            // Group expressions by their last line (matching RuboCop's expressions_per_line)
            let mut line_counts: Vec<(usize, usize)> = Vec::new();
            for expr in &body {
                let end_offset = expr.location().end_offset();
                // Use end_offset - 1 to get the line of the last byte of the expression
                let (last_line, _) = self.source.offset_to_line_col(end_offset.saturating_sub(1));
                if let Some(entry) = line_counts.last_mut() {
                    if entry.0 == last_line {
                        entry.1 += 1;
                        continue;
                    }
                }
                line_counts.push((last_line, 1));
            }

            for &(line, count) in &line_counts {
                if count >= 2 {
                    self.lines.insert(line);
                }
            }
        }

        // Continue visiting children
        ruby_prism::visit_statements_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Semicolon, "cops/style/semicolon");
}
