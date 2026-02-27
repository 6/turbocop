use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct StripHeredoc;

impl Cop for StripHeredoc {
    fn name(&self) -> &'static str {
        "Rails/StripHeredoc"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"strip_heredoc" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Only flag when the direct receiver is a heredoc.
        // In Prism, heredocs are StringNode or InterpolatedStringNode with opening starting with "<<".
        let is_heredoc = if let Some(s) = receiver.as_string_node() {
            s.opening_loc()
                .map(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
                .unwrap_or(false)
        } else if let Some(s) = receiver.as_interpolated_string_node() {
            s.opening_loc()
                .map(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
                .unwrap_or(false)
        } else {
            false
        };

        if !is_heredoc {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use squiggly heredoc (`<<~`) instead of `strip_heredoc`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StripHeredoc, "cops/rails/strip_heredoc");
}
