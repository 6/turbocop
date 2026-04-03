use std::collections::BTreeSet;

use ruby_prism::Visit;

use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Mirrors RuboCop's owner-sensitive handling of exception clauses.
///
/// The raw line scan previously treated every line-start `rescue`/`ensure`/`else`
/// as a candidate, which created false positives for `class`/`module` body
/// rescues and for `=begin` comment blocks. The fix narrows detection to keyword
/// lines collected from the Prism owners RuboCop actually checks (`def`, block,
/// and explicit `begin`) while still handling sole-body rescue modifiers and
/// skipping all non-code ranges, including comments.
pub struct EmptyLinesAroundExceptionHandlingKeywords;

const KEYWORDS: &[&[u8]] = &[b"rescue", b"ensure", b"else"];

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

#[derive(Default)]
struct ExceptionKeywordLines {
    rescue_lines: BTreeSet<usize>,
    ensure_lines: BTreeSet<usize>,
    else_lines: BTreeSet<usize>,
    rescue_modifier_lines: BTreeSet<usize>,
}

fn insert_line_for_offset(source: &SourceFile, lines: &mut BTreeSet<usize>, offset: usize) {
    let (line, _) = source.offset_to_line_col(offset);
    lines.insert(line);
}

struct ExceptionKeywordLineCollector<'a> {
    source: &'a SourceFile,
    lines: ExceptionKeywordLines,
}

impl ExceptionKeywordLineCollector<'_> {
    fn collect_begin_keywords(&mut self, begin_node: &ruby_prism::BeginNode<'_>) {
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            self.collect_rescue_chain(rescue_clause);
        }
        if let Some(else_clause) = begin_node.else_clause() {
            insert_line_for_offset(
                self.source,
                &mut self.lines.else_lines,
                else_clause.else_keyword_loc().start_offset(),
            );
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            insert_line_for_offset(
                self.source,
                &mut self.lines.ensure_lines,
                ensure_clause.ensure_keyword_loc().start_offset(),
            );
        }
    }

    fn collect_rescue_chain(&mut self, rescue_node: ruby_prism::RescueNode<'_>) {
        insert_line_for_offset(
            self.source,
            &mut self.lines.rescue_lines,
            rescue_node.keyword_loc().start_offset(),
        );

        if let Some(subsequent) = rescue_node.subsequent() {
            self.collect_rescue_chain(subsequent);
        }
    }

    fn collect_body_keywords(&mut self, body: &ruby_prism::Node<'_>) {
        if let Some(begin_node) = body.as_begin_node() {
            self.collect_begin_keywords(&begin_node);
            return;
        }

        if let Some(rescue_node) = body.as_rescue_node() {
            self.collect_rescue_chain(rescue_node);
            return;
        }

        if let Some(ensure_node) = body.as_ensure_node() {
            insert_line_for_offset(
                self.source,
                &mut self.lines.ensure_lines,
                ensure_node.ensure_keyword_loc().start_offset(),
            );
        }
    }

    fn collect_sole_body_modifier(&mut self, body: &ruby_prism::Node<'_>, owner_line: usize) {
        if let Some(line) = self.sole_body_modifier_line(body, owner_line) {
            self.lines.rescue_modifier_lines.insert(line);
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

impl<'pr> Visit<'pr> for ExceptionKeywordLineCollector<'_> {
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if let Some(begin_loc) = node.begin_keyword_loc() {
            self.collect_begin_keywords(node);

            let (owner_line, _) = self.source.offset_to_line_col(begin_loc.start_offset());
            if let Some(statements) = node.statements()
                && let Some(line) = self.sole_statement_modifier_line(statements, owner_line)
            {
                self.lines.rescue_modifier_lines.insert(line);
            }
        }

        ruby_prism::visit_begin_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let (owner_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        if let Some(body) = node.body() {
            self.collect_body_keywords(&body);
            self.collect_sole_body_modifier(&body, owner_line);
        }

        ruby_prism::visit_block_node(self, node);
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let (owner_line, _) = self
            .source
            .offset_to_line_col(node.def_keyword_loc().start_offset());
        if let Some(body) = node.body() {
            self.collect_body_keywords(&body);
            self.collect_sole_body_modifier(&body, owner_line);
        }

        ruby_prism::visit_def_node(self, node);
    }
}

fn collect_exception_keyword_lines(
    source: &SourceFile,
    parse_result: &ruby_prism::ParseResult<'_>,
) -> ExceptionKeywordLines {
    let mut collector = ExceptionKeywordLineCollector {
        source,
        lines: ExceptionKeywordLines::default(),
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
        let keyword_lines = collect_exception_keyword_lines(source, parse_result);
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

            // Skip keywords inside comments, strings, heredocs, regexps, and symbols.
            if !code_map.is_code(byte_offset + trimmed_start) {
                byte_offset += line_len;
                continue;
            }

            let keyword_allowed = if keyword == b"rescue" {
                keyword_lines.rescue_lines.contains(&line_num)
            } else if keyword == b"ensure" {
                keyword_lines.ensure_lines.contains(&line_num)
            } else {
                keyword_lines.else_lines.contains(&line_num)
            };

            if !keyword_allowed {
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

        for line_num in keyword_lines.rescue_modifier_lines {
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
