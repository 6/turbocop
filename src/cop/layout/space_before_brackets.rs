use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct SpaceBeforeBrackets;

impl Cop for SpaceBeforeBrackets {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeBrackets"
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

        let method_name = call.name().as_slice();
        if method_name != b"[]" && method_name != b"[]=" {
            return;
        }

        // Skip desugared calls like `collection.[](key)` — these have a dot
        if call.call_operator_loc().is_some() {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let receiver_end = receiver.location().end_offset();
        let selector_start = match call.opening_loc() {
            Some(loc) => loc.start_offset(),
            None => return,
        };

        // No space between receiver and `[`
        if receiver_end >= selector_start {
            return;
        }

        // Check that the gap is only whitespace (spaces/tabs)
        let bytes = source.as_bytes();
        let gap = &bytes[receiver_end..selector_start];
        if !gap.iter().all(|&b| b == b' ' || b == b'\t') {
            return;
        }

        let (line, col) = source.offset_to_line_col(receiver_end);
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            "Remove the space before the opening brackets.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeBrackets, "cops/layout/space_before_brackets");
}
