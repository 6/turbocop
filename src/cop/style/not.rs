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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
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
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `!` instead of `not`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Not, "cops/style/not");
}
