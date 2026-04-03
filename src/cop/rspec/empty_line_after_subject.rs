use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::shared::constant_predicates;
use crate::cop::shared::node_type::PROGRAM_NODE;
use crate::cop::shared::util::{
    RSPEC_DEFAULT_INCLUDE, is_blank_or_whitespace_line, is_rspec_example_group,
    is_rspec_shared_group, is_rspec_subject, line_at,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=37, FN=0.
///
/// FP=37 root cause: separator lines containing only spaces/tabs were treated as
/// non-blank by `is_blank_line`, so `subject` followed by whitespace-only lines
/// were incorrectly flagged. RuboCop's `blank?` separator check treats those
/// lines as blank.
///
/// FN=0: no missing detections were reported in corpus data for this run.
///
/// Historical parity fixes retained: top-level RSpec root scoping, recursive
/// traversal for nested include/shared-example trees, heredoc-aware end offsets,
/// and `rubocop:enable` comment-line report behavior.
///
/// Fix: use a whitespace-aware blank-line check for separator detection.
pub struct EmptyLineAfterSubject;

impl Cop for EmptyLineAfterSubject {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterSubject"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[PROGRAM_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        let (comment_lines, enable_directive_lines) = build_comment_line_sets(source, parse_result);

        // Match RuboCop's InsideExampleGroup root scoping: only process top-level
        // spec groups. Specs wrapped in module/class roots are intentionally skipped.
        for stmt in program.statements().body().iter() {
            if !is_spec_group_call(&stmt) {
                continue;
            }
            let mut visitor = SubjectSeparationVisitor {
                source,
                cop: self,
                diagnostics,
                comment_lines: &comment_lines,
                enable_directive_lines: &enable_directive_lines,
            };
            visitor.visit(&stmt);
        }
    }
}

struct SubjectSeparationVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a EmptyLineAfterSubject,
    diagnostics: &'a mut Vec<Diagnostic>,
    comment_lines: &'a HashSet<usize>,
    enable_directive_lines: &'a HashSet<usize>,
}

impl<'a> SubjectSeparationVisitor<'a> {
    fn check_subject_in_list<'pr>(
        &mut self,
        siblings: &[ruby_prism::Node<'pr>],
        idx: usize,
        subject_stmt: &ruby_prism::Node<'pr>,
        subject_call: &ruby_prism::CallNode<'pr>,
    ) {
        if idx + 1 >= siblings.len() {
            return;
        }

        let report_line = match missing_separating_line(
            self.source,
            subject_stmt,
            self.comment_lines,
            self.enable_directive_lines,
        ) {
            Some(line) => line,
            None => return,
        };

        let report_col = line_at(self.source, report_line)
            .map(|line| {
                line.iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .count()
            })
            .unwrap_or(0);

        let method_name = std::str::from_utf8(subject_call.name().as_slice()).unwrap_or("subject");
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            report_line,
            report_col,
            format!("Add an empty line after `{method_name}`."),
        ));
    }
}

impl<'a, 'pr> Visit<'pr> for SubjectSeparationVisitor<'a> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let siblings: Vec<_> = node.body().iter().collect();

        for (idx, stmt) in siblings.iter().enumerate() {
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            if call.receiver().is_some() || !is_rspec_subject(call.name().as_slice()) {
                continue;
            }

            if call.block().is_none() {
                continue;
            }

            self.check_subject_in_list(&siblings, idx, stmt, &call);
        }

        ruby_prism::visit_statements_node(self, node);
    }
}

fn is_spec_group_call(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    let name = call.name().as_slice();
    if let Some(recv) = call.receiver() {
        constant_predicates::constant_short_name(&recv).is_some_and(|n| n == b"RSpec")
            && (is_rspec_example_group(name) || is_rspec_shared_group(name))
    } else {
        is_rspec_example_group(name) || is_rspec_shared_group(name)
    }
}

fn missing_separating_line(
    source: &SourceFile,
    subject_stmt: &ruby_prism::Node<'_>,
    comment_lines: &HashSet<usize>,
    enable_directive_lines: &HashSet<usize>,
) -> Option<usize> {
    // Match RuboCop's FinalEndLocation mixin: multiline subject bodies containing
    // heredocs may end after the call node's own location.
    let loc = subject_stmt.location();
    let mut max_end_offset = loc.end_offset();
    let heredoc_max = find_max_heredoc_end_offset(source, subject_stmt);
    if heredoc_max > max_end_offset {
        max_end_offset = heredoc_max;
    }
    let end_offset = max_end_offset.saturating_sub(1).max(loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(end_offset);

    // RuboCop's EmptyLineSeparation:
    // - allow directly-following comment lines,
    // - if the next non-comment line is blank, it's fine,
    // - otherwise report (on enable directive line when present).
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
    crate::cop_fixture_tests!(EmptyLineAfterSubject, "cops/rspec/empty_line_after_subject");
}
