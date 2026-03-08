use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::util::{
    RSPEC_DEFAULT_INCLUDE, is_blank_line, is_rspec_hook, line_at, node_on_single_line,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-07)
///
/// Corpus oracle reported FP=31, FN=18.
///
/// FP root cause: one-line hook forms with heredoc arguments were treated as
/// ending on the opening line (e.g., `before { call(<<~TXT) }`), so heredoc
/// content lines appeared to violate separation.
///
/// FN root cause: offenses that should be reported on `# rubocop:enable ...`
/// directive lines were emitted on hook lines instead (line mismatch), and
/// heredoc terminator-line offenses were missed due incorrect end-line tracking.
///
/// Fix: use heredoc-aware final-end offsets and RuboCop-equivalent comment-line
/// scanning based on Prism comments, including inline comments and
/// `rubocop:enable` directive report-line behavior.
pub struct EmptyLineAfterHook;

impl Cop for EmptyLineAfterHook {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterHook"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_consecutive = config.get_bool("AllowConsecutiveOneLiners", true);
        let (comment_lines, enable_directive_lines) = build_comment_line_sets(source, parse_result);
        let mut visitor = HookSeparationVisitor {
            source,
            cop: self,
            diagnostics,
            allow_consecutive,
            comment_lines: &comment_lines,
            enable_directive_lines: &enable_directive_lines,
        };
        visitor.visit(&parse_result.node());
    }
}

struct HookSeparationVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a EmptyLineAfterHook,
    diagnostics: &'a mut Vec<Diagnostic>,
    allow_consecutive: bool,
    comment_lines: &'a HashSet<usize>,
    enable_directive_lines: &'a HashSet<usize>,
}

impl<'a, 'pr> Visit<'pr> for HookSeparationVisitor<'a> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let nodes: Vec<_> = node.body().iter().collect();

        for (i, stmt) in nodes.iter().enumerate() {
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            if call.receiver().is_some() || !is_rspec_hook(call.name().as_slice()) {
                continue;
            }
            if call.block().is_none() {
                continue;
            }

            if i + 1 >= nodes.len() {
                continue;
            }

            let loc = stmt.location();
            if self.allow_consecutive && node_on_single_line(self.source, &loc) {
                let next_stmt = &nodes[i + 1];
                if let Some(next_call) = next_stmt.as_call_node() {
                    if next_call.receiver().is_none()
                        && is_rspec_hook(next_call.name().as_slice())
                        && next_call.block().is_some()
                        && node_on_single_line(self.source, &next_stmt.location())
                    {
                        continue;
                    }
                }
            }

            let report_line = match missing_separating_line(
                self.source,
                stmt,
                self.comment_lines,
                self.enable_directive_lines,
            ) {
                Some(line) => line,
                None => continue,
            };

            let report_col = line_at(self.source, report_line)
                .map(|line| line.iter().take_while(|&&b| b == b' ').count())
                .unwrap_or(0);

            let hook_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("before");
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                report_line,
                report_col,
                format!("Add an empty line after `{hook_name}`."),
            ));
        }

        ruby_prism::visit_statements_node(self, node);
    }
}

fn missing_separating_line(
    source: &SourceFile,
    hook_stmt: &ruby_prism::Node<'_>,
    comment_lines: &HashSet<usize>,
    enable_directive_lines: &HashSet<usize>,
) -> Option<usize> {
    let loc = hook_stmt.location();
    let mut max_end_offset = loc.end_offset();
    let heredoc_max = find_max_heredoc_end_offset(source, hook_stmt);
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
        Some(next_line) if is_blank_line(next_line) => None,
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
    crate::cop_fixture_tests!(EmptyLineAfterHook, "cops/rspec/empty_line_after_hook");
}
