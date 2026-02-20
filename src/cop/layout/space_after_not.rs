use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct SpaceAfterNot;

impl Cop for SpaceAfterNot {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterNot"
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
        // CallNode with method name "!" and a receiver
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if call.name().as_slice() != b"!" || call.receiver().is_none() {
            return;
        }
        // Check if there's a space between ! and the receiver
        let bang_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return,
        };
        let bang_end = bang_loc.end_offset();
        let recv_start = call.receiver().unwrap().location().start_offset();
        if recv_start > bang_end {
            let between = &source.as_bytes()[bang_end..recv_start];
            if between.iter().any(|&b| b == b' ' || b == b'\t') {
                let (line, column) = source.offset_to_line_col(bang_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not leave space between `!` and its argument.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAfterNot, "cops/layout/space_after_not");
}
