use crate::cop::shared::node_type::{CALL_NODE, CALL_OPERATOR_WRITE_NODE, CONSTANT_PATH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// RuboCop still flags spaces after `.`/`&.` when Prism represents the send as
/// an implicit `.()` call with no `message_loc`, and when compound writes like
/// `obj.  attr += 1` are emitted as `CallOperatorWriteNode`. This cop now falls
/// back to `opening_loc()` for implicit calls and checks compound write nodes.
pub struct SpaceAroundMethodCallOperator;

impl Cop for SpaceAroundMethodCallOperator {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundMethodCallOperator"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CALL_OPERATOR_WRITE_NODE, CONSTANT_PATH_NODE]
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
        // Handle CallNode (method calls with . or &.)
        if let Some(call) = node.as_call_node() {
            check_call_operator_spacing(
                self,
                source,
                call.receiver()
                    .map(|receiver| receiver.location().end_offset()),
                call.call_operator_loc(),
                call.message_loc()
                    .map(|loc| loc.start_offset())
                    .or_else(|| call.opening_loc().map(|loc| loc.start_offset())),
                diagnostics,
            );
        }

        if let Some(write) = node.as_call_operator_write_node() {
            check_call_operator_spacing(
                self,
                source,
                write
                    .receiver()
                    .map(|receiver| receiver.location().end_offset()),
                write.call_operator_loc(),
                write.message_loc().map(|loc| loc.start_offset()),
                diagnostics,
            );
        }

        // Handle ConstantPathNode (:: operator)
        if let Some(cp) = node.as_constant_path_node() {
            // Only check when there's a name (e.g., `Foo::Bar`, not bare `::`)
            if cp.name().is_some() {
                let delim_loc = cp.delimiter_loc();
                let delim_end = delim_loc.end_offset();
                let name_loc = cp.name_loc();
                let name_start = name_loc.start_offset();
                if name_start > delim_end {
                    let bytes = &source.as_bytes()[delim_end..name_start];
                    if bytes.iter().all(|&b| b == b' ' || b == b'\t') && !bytes.is_empty() {
                        let (delim_line, _) = source.offset_to_line_col(delim_end);
                        let (name_line, _) = source.offset_to_line_col(name_start);
                        if delim_line == name_line {
                            let (line, col) = source.offset_to_line_col(delim_end);
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                col,
                                "Avoid using spaces around a method call operator.".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }
}

fn check_call_operator_spacing(
    cop: &dyn Cop,
    source: &SourceFile,
    receiver_end: Option<usize>,
    dot_loc: Option<ruby_prism::Location<'_>>,
    selector_start: Option<usize>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(dot_loc) = dot_loc else {
        return;
    };

    if !matches!(dot_loc.as_slice(), b"." | b"&.") {
        return;
    }

    if let Some(receiver_end) = receiver_end {
        push_spacing_offense(
            cop,
            source,
            receiver_end,
            dot_loc.start_offset(),
            diagnostics,
        );
    }

    if let Some(selector_start) = selector_start {
        push_spacing_offense(
            cop,
            source,
            dot_loc.end_offset(),
            selector_start,
            diagnostics,
        );
    }
}

fn push_spacing_offense(
    cop: &dyn Cop,
    source: &SourceFile,
    gap_start: usize,
    gap_end: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if gap_end <= gap_start {
        return;
    }

    let bytes = &source.as_bytes()[gap_start..gap_end];
    if bytes.is_empty() || !bytes.iter().all(|&b| b == b' ' || b == b'\t') {
        return;
    }

    let (gap_start_line, gap_start_col) = source.offset_to_line_col(gap_start);
    let (gap_end_line, _) = source.offset_to_line_col(gap_end);
    if gap_start_line != gap_end_line {
        return;
    }

    diagnostics.push(cop.diagnostic(
        source,
        gap_start_line,
        gap_start_col,
        "Avoid using spaces around a method call operator.".to_string(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceAroundMethodCallOperator,
        "cops/layout/space_around_method_call_operator"
    );
}
