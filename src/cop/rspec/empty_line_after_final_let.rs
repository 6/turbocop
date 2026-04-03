use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::shared::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_blank_or_whitespace_line, is_rspec_example_group, is_rspec_let,
    is_rspec_shared_group, line_at,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/EmptyLineAfterFinalLet
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=192, FN=0.
///
/// Previous fix: RuboCop's `example_group_with_body?` only matches ExampleGroups
/// (describe/context/feature/etc.) and NOT SharedGroups
/// (`shared_examples`/`shared_examples_for`/`shared_context`). Excluding shared
/// groups removed a large FP cluster.
///
/// FP=192 root cause: separator lines containing only spaces/tabs were treated
/// as non-blank by `is_blank_line`, so `let` followed by whitespace-only line
/// and then `before/it` triggered false positives. RuboCop's separation logic
/// uses `blank?`, which treats whitespace-only lines as blank.
///
/// FN=0: no missing detections were reported in corpus data for this run.
///
/// Fix: keep strict blank semantics globally, but for this separation cop use a
/// whitespace-aware blank-line check when deciding whether a separator exists.
///
/// ## Corpus investigation (2026-03-25)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// FP 1 (gocardless/nandi line 94): form-feed character (0x0C) in separator
/// line after let. Already handled by `is_blank_or_whitespace_line` — verified
/// FIXED in `verify_cop_locations.py`.
///
/// FP 2 (liaoziyang/stackneveroverflow line 637): `let(:bar)` without a block,
/// inside a proc/describe chain in a test. RuboCop's `let?` matcher requires
/// either a block `(block (send nil? ...))` or a block_pass argument
/// `(send nil? ... _ block_pass)`. A bare `let(:name)` call is not a real let
/// definition. Fix: require `c.block().is_some()` when identifying let calls.
pub struct EmptyLineAfterFinalLet;

impl Cop for EmptyLineAfterFinalLet {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterFinalLet"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let (comment_lines, enable_directive_lines) = build_comment_line_sets(source, parse_result);
        let mut visitor = FinalLetVisitor {
            source,
            cop: self,
            diagnostics,
            comment_lines: &comment_lines,
            enable_directive_lines: &enable_directive_lines,
        };
        visitor.visit(&parse_result.node());
    }
}

struct FinalLetVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a EmptyLineAfterFinalLet,
    diagnostics: &'a mut Vec<Diagnostic>,
    comment_lines: &'a HashSet<usize>,
    enable_directive_lines: &'a HashSet<usize>,
}

impl<'a, 'pr> Visit<'pr> for FinalLetVisitor<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if !is_example_group_call(node) {
            ruby_prism::visit_call_node(self, node);
            return;
        }

        let block = match node.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            },
            None => {
                ruby_prism::visit_call_node(self, node);
                return;
            }
        };

        let body = match block.body() {
            Some(b) => b,
            None => {
                ruby_prism::visit_call_node(self, node);
                return;
            }
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => {
                ruby_prism::visit_call_node(self, node);
                return;
            }
        };

        // Find the last let/let! in this block.
        // Match RuboCop's `let?` which requires either a block or block_pass:
        //   (block (send nil? #Helpers.all ...) ...)
        //   (send nil? #Helpers.all _ block_pass)
        // A bare `let(:name)` without a block is NOT a real let definition.
        let nodes: Vec<_> = stmts.body().iter().collect();
        let mut last_let_idx = None;
        for (i, stmt) in nodes.iter().enumerate() {
            if let Some(c) = stmt.as_call_node() {
                if c.receiver().is_none()
                    && is_rspec_let(c.name().as_slice())
                    && c.block().is_some()
                {
                    last_let_idx = Some(i);
                }
            }
        }

        let last_idx = match last_let_idx {
            Some(i) => i,
            None => {
                ruby_prism::visit_call_node(self, node);
                return;
            }
        };

        // Check if there's a next statement after the last let
        if last_idx + 1 >= nodes.len() {
            ruby_prism::visit_call_node(self, node);
            return; // let is the last statement
        }

        let last_let = &nodes[last_idx];
        let report_line = match missing_separating_line(
            self.source,
            last_let,
            self.comment_lines,
            self.enable_directive_lines,
        ) {
            Some(line) => line,
            None => {
                ruby_prism::visit_call_node(self, node);
                return;
            }
        };

        let let_name = if let Some(c) = last_let.as_call_node() {
            std::str::from_utf8(c.name().as_slice()).unwrap_or("let")
        } else {
            "let"
        };

        let report_col = if let Some(line_bytes) = line_at(self.source, report_line) {
            line_bytes.iter().take_while(|&&b| b == b' ').count()
        } else {
            0
        };

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            report_line,
            report_col,
            format!("Add an empty line after the last `{let_name}`."),
        ));

        ruby_prism::visit_call_node(self, node);
    }
}

fn is_example_group_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let method_name = call.name().as_slice();
    if let Some(recv) = call.receiver() {
        util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
            && is_rspec_example_group(method_name)
            && !is_rspec_shared_group(method_name)
    } else {
        is_rspec_example_group(method_name) && !is_rspec_shared_group(method_name)
    }
}

fn missing_separating_line(
    source: &SourceFile,
    last_let: &ruby_prism::Node<'_>,
    comment_lines: &HashSet<usize>,
    enable_directive_lines: &HashSet<usize>,
) -> Option<usize> {
    // Match RuboCop's FinalEndLocation to handle heredoc lets.
    let loc = last_let.location();
    let mut max_end_offset = loc.end_offset();
    let heredoc_max = find_max_heredoc_end_offset(source, last_let);
    if heredoc_max > max_end_offset {
        max_end_offset = heredoc_max;
    }
    let end_offset = max_end_offset.saturating_sub(1).max(loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(end_offset);

    let mut line = end_line;
    let mut enable_directive_line = None;
    while comment_lines.contains(&(line + 1)) {
        line += 1;
        if enable_directive_lines.contains(&line) {
            enable_directive_line = Some(line);
        }
    }

    match line_at(source, line + 1) {
        Some(next_line) if is_blank_or_whitespace_line(next_line) => None,
        Some(_) => Some(enable_directive_line.unwrap_or(end_line)),
        None => None,
    }
}

fn build_comment_line_sets(
    source: &SourceFile,
    parse_result: &ruby_prism::ParseResult<'_>,
) -> (HashSet<usize>, HashSet<usize>) {
    let mut comment_lines = HashSet::new();
    let mut enable_directive_lines = HashSet::new();

    for comment in parse_result.comments() {
        let loc = comment.location();
        let (start_line, _) = source.offset_to_line_col(loc.start_offset());
        let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_offset);

        for line in start_line..=end_line {
            comment_lines.insert(line);
        }

        let comment_bytes = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        if comment_bytes
            .windows(b"rubocop:enable".len())
            .any(|window| window == b"rubocop:enable")
        {
            enable_directive_lines.insert(start_line);
        }
    }

    (comment_lines, enable_directive_lines)
}

fn find_max_heredoc_end_offset(source: &SourceFile, node: &ruby_prism::Node<'_>) -> usize {
    struct MaxHeredocVisitor<'a> {
        source: &'a SourceFile,
        max_offset: usize,
    }

    impl<'pr> Visit<'pr> for MaxHeredocVisitor<'_> {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            if let Some(opening) = node.opening_loc() {
                let bytes = &self.source.as_bytes()[opening.start_offset()..opening.end_offset()];
                if bytes.starts_with(b"<<") {
                    if let Some(closing) = node.closing_loc() {
                        self.max_offset = self.max_offset.max(closing.end_offset());
                    }
                    return;
                }
            }
            ruby_prism::visit_string_node(self, node);
        }

        fn visit_interpolated_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedStringNode<'pr>,
        ) {
            if let Some(opening) = node.opening_loc() {
                let bytes = &self.source.as_bytes()[opening.start_offset()..opening.end_offset()];
                if bytes.starts_with(b"<<") {
                    if let Some(closing) = node.closing_loc() {
                        self.max_offset = self.max_offset.max(closing.end_offset());
                    }
                    return;
                }
            }
            ruby_prism::visit_interpolated_string_node(self, node);
        }
    }

    let mut visitor = MaxHeredocVisitor {
        source,
        max_offset: 0,
    };
    visitor.visit(node);
    visitor.max_offset
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        EmptyLineAfterFinalLet,
        "cops/rspec/empty_line_after_final_let"
    );
}
