use ruby_prism::Visit;

use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that method calls on HEREDOC receivers are on the same line as the opening.
///
/// Prism keeps HEREDOC content and terminators outside the string node's main
/// `location()`. That meant `<<-'SQL' % [ ... ]` was missed because the `%`
/// token stayed on the opening line while the argument list continued after the
/// terminator. Match RuboCop by treating the call as misplaced whenever the
/// overall call span extends past the HEREDOC closing.
pub struct HeredocMethodCallPosition;

impl Cop for HeredocMethodCallPosition {
    fn name(&self) -> &'static str {
        "Lint/HeredocMethodCallPosition"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let mut visitor = HeredocVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct HeredocVisitor<'a, 'src> {
    cop: &'a HeredocMethodCallPosition,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for HeredocVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(recv) = node.receiver() {
            if let Some(heredoc_end_offset) = heredoc_end_offset(&recv) {
                let call_end_offset = node.location().end_offset();

                if call_end_offset > heredoc_end_offset {
                    let offense_offset = node
                        .message_loc()
                        .filter(|loc| loc.start_offset() >= heredoc_end_offset)
                        .map(|loc| loc.start_offset())
                        .unwrap_or_else(|| {
                            util::first_non_whitespace_offset(
                                self.source.as_bytes(),
                                heredoc_end_offset,
                            )
                            .unwrap_or(heredoc_end_offset)
                        });
                    let (line, column) = self.source.offset_to_line_col(offense_offset);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Put a method call with a HEREDOC receiver on the same line as the HEREDOC opening.".to_string(),
                    ));
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

fn heredoc_end_offset(node: &ruby_prism::Node<'_>) -> Option<usize> {
    if let Some(str_node) = node.as_interpolated_string_node() {
        if let Some(open) = str_node.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                return str_node.closing_loc().map(|loc| loc.end_offset());
            }
        }
    }
    if let Some(str_node) = node.as_string_node() {
        if let Some(open) = str_node.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                return str_node.closing_loc().map(|loc| loc.end_offset());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        HeredocMethodCallPosition,
        "cops/lint/heredoc_method_call_position"
    );
}
