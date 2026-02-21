use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct Not;

impl Cop for Not {
    fn name(&self) -> &'static str {
        "Style/Not"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // `not x` parses as a CallNode with name `!` in Prism
        if call_node.name().as_slice() != b"!" {
            return;
        }

        // Distinguish `not` from `!` by checking the source text at the message_loc
        let msg_loc = match call_node.message_loc() {
            Some(loc) => loc,
            None => return,
        };

        if msg_loc.as_slice() != b"not" {
            return;
        }

        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        let mut diag = self.diagnostic(
            source, line, column,
            "Use `!` instead of `not`.".to_string(),
        );
        if let Some(ref mut corr) = corrections {
            // Replace `not` and any trailing space with `!`
            let not_end = msg_loc.end_offset();
            let bytes = source.as_bytes();
            let replace_end = if not_end < bytes.len() && bytes[not_end] == b' ' {
                not_end + 1
            } else {
                not_end
            };
            corr.push(crate::correction::Correction {
                start: msg_loc.start_offset(), end: replace_end,
                replacement: "!".to_string(),
                cop_name: self.name(), cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Not, "cops/style/not");
    crate::cop_autocorrect_fixture_tests!(Not, "cops/style/not");
}
