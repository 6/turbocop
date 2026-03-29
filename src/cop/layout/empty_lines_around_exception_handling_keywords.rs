use std::collections::BTreeSet;

use ruby_prism::Visit;

use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-29)
///
/// Cached corpus oracle reported FP=3, FN=5.
///
/// Fixed FN=5 from three Prism-specific gaps:
/// - postfix `expr rescue nil` is a `RescueModifierNode`, so the line-start scan
///   missed rescue keywords that do not begin the line;
/// - the line matcher skipped valid headers written as `rescue(EOFError)` and
///   `rescue; []` because it only accepted whitespace or `=>` after `rescue`;
/// - RuboCop skips same-line `rescue ... end` clauses entirely, so the inline
///   `end` guard must suppress both the "before" and "after" checks, not only
///   the trailing blank-line check.
///
/// Fixed FP regression found during the required sample rerun:
/// - RuboCop only treats postfix rescue modifiers like exception-handling
///   keywords when the modifier is the sole body expression of a def/block/begin;
///   a later statement-level scan was too broad and falsely flagged blank lines
///   before modifiers that appeared alongside sibling statements;
/// - RuboCop does not treat blank lines after postfix rescue modifiers as
///   offenses, only blank lines before them. The modifier path therefore only
///   performs the leading-blank check.
///
/// Earlier accepted fixes retained support for compact `rescue=>e` headers and
/// skipped heredoc/string content. This patch keeps that behavior and extends it
/// only to the narrow additional forms RuboCop accepts.
pub struct EmptyLinesAroundExceptionHandlingKeywords;

const KEYWORDS: &[&[u8]] = &[b"rescue", b"ensure", b"else"];

/// Check if an `else` on this line is part of a rescue block (not if/case/etc.).
/// Scan backwards from the `else` to find whether we hit `rescue` (rescue-else)
/// or `if`/`unless`/`case`/`when`/`elsif` (regular else) at the same indentation.
fn is_rescue_else(lines: &[&[u8]], else_idx: usize, else_indent: usize) -> bool {
    for i in (0..else_idx).rev() {
        let line = lines[i];
        let start = match line.iter().position(|&b| b != b' ' && b != b'\t') {
            Some(p) => p,
            None => continue,
        };
        let content = &line[start..];
        // Only consider lines at the same or less indentation
        if start > else_indent {
            continue;
        }
        // Check for rescue at the same indent
        if start == else_indent && starts_with_kw(content, b"rescue") {
            return true;
        }
        // If we hit a structural keyword at the same or less indentation, it's not rescue-else
        if starts_with_kw(content, b"if")
            || starts_with_kw(content, b"unless")
            || starts_with_kw(content, b"case")
            || starts_with_kw(content, b"when")
            || starts_with_kw(content, b"elsif")
        {
            return false;
        }
        // def/begin/class/module at same or less indent = scope boundary, check if rescue exists
        if starts_with_kw(content, b"def")
            || starts_with_kw(content, b"begin")
            || starts_with_kw(content, b"class")
            || starts_with_kw(content, b"module")
        {
            return false;
        }
    }
    false
}

fn starts_with_kw(content: &[u8], kw: &[u8]) -> bool {
    content.starts_with(kw)
        && (content.len() == kw.len()
            || !content[kw.len()].is_ascii_alphanumeric() && content[kw.len()] != b'_')
}

fn matches_keyword_line(content: &[u8], kw: &[u8]) -> bool {
    if !content.starts_with(kw) {
        return false;
    }

    let Some(rest) = content.get(kw.len()..) else {
        return true;
    };

    rest.is_empty()
        || matches!(rest[0], b' ' | b'\t' | b'\n' | b'\r' | b';')
        || (kw == b"rescue" && (rest.starts_with(b"=>") || rest[0] == b'('))
}

fn has_inline_end(content: &[u8], keyword: &[u8]) -> bool {
    let Some(rest) = content.get(keyword.len()..) else {
        return false;
    };

    for idx in 0..rest.len() {
        if starts_with_kw(&rest[idx..], b"end") {
            return true;
        }
    }

    false
}

struct RescueModifierLineCollector<'a> {
    source: &'a SourceFile,
    lines: BTreeSet<usize>,
}

impl RescueModifierLineCollector<'_> {
    fn collect_sole_body_modifier(&mut self, body: &ruby_prism::Node<'_>, owner_line: usize) {
        if let Some(line) = self.sole_body_modifier_line(body, owner_line) {
            self.lines.insert(line);
        }
    }

    fn sole_body_modifier_line(
        &self,
        body: &ruby_prism::Node<'_>,
        owner_line: usize,
    ) -> Option<usize> {
        if let Some(rescue_modifier) = body.as_rescue_modifier_node() {
            return self.modifier_line(rescue_modifier, owner_line);
        }

        if let Some(statements) = body.as_statements_node() {
            return self.sole_statement_modifier_line(statements, owner_line);
        }

        let begin_node = body.as_begin_node()?;
        let statements = begin_node.statements()?;
        self.sole_statement_modifier_line(statements, owner_line)
    }

    fn sole_statement_modifier_line(
        &self,
        statements: ruby_prism::StatementsNode<'_>,
        owner_line: usize,
    ) -> Option<usize> {
        let body = statements.body();
        if body.len() != 1 {
            return None;
        }

        let rescue_modifier = body.first()?.as_rescue_modifier_node()?;
        self.modifier_line(rescue_modifier, owner_line)
    }

    fn modifier_line(
        &self,
        rescue_modifier: ruby_prism::RescueModifierNode<'_>,
        owner_line: usize,
    ) -> Option<usize> {
        let (line, _) = self
            .source
            .offset_to_line_col(rescue_modifier.keyword_loc().start_offset());
        (line != owner_line).then_some(line)
    }
}

impl<'pr> Visit<'pr> for RescueModifierLineCollector<'_> {
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if let Some(begin_loc) = node.begin_keyword_loc() {
            let (owner_line, _) = self.source.offset_to_line_col(begin_loc.start_offset());
            if let Some(statements) = node.statements() {
                if let Some(line) = self.sole_statement_modifier_line(statements, owner_line) {
                    self.lines.insert(line);
                }
            }
        }

        ruby_prism::visit_begin_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let (owner_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        if let Some(body) = node.body() {
            self.collect_sole_body_modifier(&body, owner_line);
        }

        ruby_prism::visit_block_node(self, node);
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let (owner_line, _) = self
            .source
            .offset_to_line_col(node.def_keyword_loc().start_offset());
        if let Some(body) = node.body() {
            self.collect_sole_body_modifier(&body, owner_line);
        }

        ruby_prism::visit_def_node(self, node);
    }
}

fn collect_rescue_modifier_keyword_lines(
    source: &SourceFile,
    parse_result: &ruby_prism::ParseResult<'_>,
) -> BTreeSet<usize> {
    let mut collector = RescueModifierLineCollector {
        source,
        lines: BTreeSet::new(),
    };
    collector.visit(&parse_result.node());
    collector.lines
}

impl Cop for EmptyLinesAroundExceptionHandlingKeywords {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundExceptionHandlingKeywords"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let lines: Vec<&[u8]> = source.lines().collect();
        let rescue_modifier_keyword_lines =
            collect_rescue_modifier_keyword_lines(source, parse_result);
        let mut byte_offset: usize = 0;

        for (i, line) in lines.iter().enumerate() {
            let line_len = line.len() + 1; // +1 for newline
            let line_num = i + 1;
            let trimmed_start = match line.iter().position(|&b| b != b' ' && b != b'\t') {
                Some(p) => p,
                None => {
                    byte_offset += line_len;
                    continue;
                }
            };
            let content = &line[trimmed_start..];

            // Check if this line is a rescue/ensure/else keyword at the start of a line
            let matched_keyword = KEYWORDS
                .iter()
                .find(|&&kw| matches_keyword_line(content, kw));

            let keyword = match matched_keyword {
                Some(kw) => *kw,
                None => {
                    byte_offset += line_len;
                    continue;
                }
            };

            // Skip keywords inside strings/heredocs/regexps/symbols
            if !code_map.is_not_string(byte_offset + trimmed_start) {
                byte_offset += line_len;
                continue;
            }

            // For `else`, only flag if it's part of a rescue block (not if/case/etc.)
            if keyword == b"else" && !is_rescue_else(&lines, i, trimmed_start) {
                byte_offset += line_len;
                continue;
            }

            let kw_str = std::str::from_utf8(keyword).unwrap_or("rescue");

            // RuboCop ignores same-line `rescue ... end` / `ensure ... end`
            // clauses entirely, not just the trailing blank after them.
            if has_inline_end(content, keyword) {
                byte_offset += line_len;
                continue;
            }

            // Check for empty line BEFORE the keyword
            if line_num >= 3 {
                let above_idx = i - 1; // 0-indexed
                if above_idx < lines.len() && util::is_blank_line(lines[above_idx]) {
                    let mut diag = self.diagnostic(
                        source,
                        line_num - 1,
                        0,
                        format!("Extra empty line detected before the `{kw_str}`."),
                    );
                    if let Some(ref mut corr) = corrections {
                        // Delete the blank line (line_num - 1 is 1-based)
                        if let (Some(start), Some(end)) = (
                            source.line_col_to_offset(line_num - 1, 0),
                            source.line_col_to_offset(line_num, 0),
                        ) {
                            corr.push(crate::correction::Correction {
                                start,
                                end,
                                replacement: String::new(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                    }
                    diagnostics.push(diag);
                }
            }

            // Check for empty line AFTER the keyword
            let below_idx = i + 1; // 0-indexed for line after
            if below_idx < lines.len() && util::is_blank_line(lines[below_idx]) {
                let mut diag = self.diagnostic(
                    source,
                    line_num + 1,
                    0,
                    format!("Extra empty line detected after the `{kw_str}`."),
                );
                if let Some(ref mut corr) = corrections {
                    // Delete the blank line (line_num + 1 is 1-based)
                    if let (Some(start), Some(end)) = (
                        source.line_col_to_offset(line_num + 1, 0),
                        source.line_col_to_offset(line_num + 2, 0),
                    ) {
                        corr.push(crate::correction::Correction {
                            start,
                            end,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                }
                diagnostics.push(diag);
            }

            byte_offset += line_len;
        }

        for line_num in rescue_modifier_keyword_lines {
            if line_num >= 3 {
                let above_idx = line_num - 2;
                if above_idx < lines.len() && util::is_blank_line(lines[above_idx]) {
                    let mut diag = self.diagnostic(
                        source,
                        line_num - 1,
                        0,
                        "Extra empty line detected before the `rescue`.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        if let (Some(start), Some(end)) = (
                            source.line_col_to_offset(line_num - 1, 0),
                            source.line_col_to_offset(line_num, 0),
                        ) {
                            corr.push(crate::correction::Correction {
                                start,
                                end,
                                replacement: String::new(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                    }
                    diagnostics.push(diag);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        EmptyLinesAroundExceptionHandlingKeywords,
        "cops/layout/empty_lines_around_exception_handling_keywords"
    );
    crate::cop_autocorrect_fixture_tests!(
        EmptyLinesAroundExceptionHandlingKeywords,
        "cops/layout/empty_lines_around_exception_handling_keywords"
    );

    #[test]
    fn skip_keywords_in_heredoc() {
        let source =
            b"x = <<~RUBY\n  begin\n    something\n\n  rescue\n\n    handle\n  end\nRUBY\n";
        let diags = run_cop_full(&EmptyLinesAroundExceptionHandlingKeywords, source);
        assert!(
            diags.is_empty(),
            "Should not fire on rescue inside heredoc, got: {:?}",
            diags
        );
    }

    #[test]
    fn skip_keywords_in_string() {
        let source = b"x = \"rescue\"\ny = 'ensure'\n";
        let diags = run_cop_full(&EmptyLinesAroundExceptionHandlingKeywords, source);
        assert!(
            diags.is_empty(),
            "Should not fire on keywords inside strings"
        );
    }
}
