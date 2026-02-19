use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that method calls on HEREDOC receivers are on the same line as the opening.
pub struct HeredocMethodCallPosition;

impl Cop for HeredocMethodCallPosition {
    fn name(&self) -> &'static str {
        "Lint/HeredocMethodCallPosition"
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
        // Check if the receiver is a heredoc
        if let Some(recv) = node.receiver() {
            if is_heredoc(&recv) {
                // The method call should be on the same line as the heredoc opening
                let heredoc_opening_line = self.source.offset_to_line_col(recv.location().start_offset()).0;

                // The message (method name) should also be on the same line
                if let Some(msg_loc) = node.message_loc() {
                    let method_line = self.source.offset_to_line_col(msg_loc.start_offset()).0;

                    if method_line != heredoc_opening_line {
                        let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Put a method call with a HEREDOC receiver on the same line as the HEREDOC opening.".to_string(),
                        ));
                    }
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

fn is_heredoc(node: &ruby_prism::Node<'_>) -> bool {
    // In Prism, heredocs are InterpolatedStringNode or StringNode with a heredoc opening
    if let Some(str_node) = node.as_interpolated_string_node() {
        if let Some(open) = str_node.opening_loc() {
            let open_bytes = open.as_slice();
            return open_bytes.starts_with(b"<<");
        }
    }
    if let Some(str_node) = node.as_string_node() {
        if let Some(open) = str_node.opening_loc() {
            let open_bytes = open.as_slice();
            return open_bytes.starts_with(b"<<");
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        HeredocMethodCallPosition,
        "cops/lint/heredoc_method_call_position"
    );
}
