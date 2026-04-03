use crate::cop::shared::node_type::{
    CALL_AND_WRITE_NODE, CALL_NODE, CALL_OPERATOR_WRITE_NODE, CALL_OR_WRITE_NODE,
    INDEX_AND_WRITE_NODE, INDEX_OPERATOR_WRITE_NODE, INDEX_OR_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-29)
///
/// Corpus oracle reported FP=0, FN=1.
///
/// FP=0: no corpus false positives are currently known.
///
/// FN=1: `SUSE__machinery__e41b642` split a chained `[]` call across a
/// backslash-newline continuation:
/// `description_hash["patterns"]["_attributes"] \` then `["patterns_system"]`.
/// Prism still exposes the second access as a `CallNode` with `name == "[]"`,
/// but the gap between the receiver end and `opening_loc` is `" \\\n  "` rather
/// than plain spaces. The previous implementation only accepted spaces/tabs, so
/// it missed this RuboCop offense. This cop now treats a single escaped newline
/// surrounded by horizontal space as the same receiver-to-bracket gap.
pub struct SpaceBeforeBrackets;

impl Cop for SpaceBeforeBrackets {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeBrackets"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_AND_WRITE_NODE,
            CALL_NODE,
            CALL_OPERATOR_WRITE_NODE,
            CALL_OR_WRITE_NODE,
            INDEX_AND_WRITE_NODE,
            INDEX_OPERATOR_WRITE_NODE,
            INDEX_OR_WRITE_NODE,
        ]
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
        if let Some(call) = node.as_call_node() {
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

            check_receiver_gap_before_brackets(
                self,
                source,
                receiver.location().end_offset(),
                call.opening_loc().map(|loc| loc.start_offset()),
                diagnostics,
            );
            return;
        }

        if let Some(write) = node.as_index_operator_write_node() {
            let receiver = match write.receiver() {
                Some(receiver) => receiver,
                None => return,
            };
            check_receiver_gap_before_brackets(
                self,
                source,
                receiver.location().end_offset(),
                Some(write.opening_loc().start_offset()),
                diagnostics,
            );
            return;
        }

        if let Some(write) = node.as_index_and_write_node() {
            let receiver = match write.receiver() {
                Some(receiver) => receiver,
                None => return,
            };
            check_receiver_gap_before_brackets(
                self,
                source,
                receiver.location().end_offset(),
                Some(write.opening_loc().start_offset()),
                diagnostics,
            );
            return;
        }

        if let Some(write) = node.as_index_or_write_node() {
            let receiver = match write.receiver() {
                Some(receiver) => receiver,
                None => return,
            };
            check_receiver_gap_before_brackets(
                self,
                source,
                receiver.location().end_offset(),
                Some(write.opening_loc().start_offset()),
                diagnostics,
            );
            return;
        }

        if let Some(write) = node.as_call_operator_write_node() {
            if write.read_name().as_slice() != b"[]" || write.call_operator_loc().is_some() {
                return;
            }
            let receiver = match write.receiver() {
                Some(receiver) => receiver,
                None => return,
            };
            check_receiver_gap_before_scanned_brackets(
                self,
                source,
                receiver.location().end_offset(),
                write.location().end_offset(),
                diagnostics,
            );
            return;
        }

        if let Some(write) = node.as_call_and_write_node() {
            if write.read_name().as_slice() != b"[]" || write.call_operator_loc().is_some() {
                return;
            }
            let receiver = match write.receiver() {
                Some(receiver) => receiver,
                None => return,
            };
            check_receiver_gap_before_scanned_brackets(
                self,
                source,
                receiver.location().end_offset(),
                write.location().end_offset(),
                diagnostics,
            );
            return;
        }

        if let Some(write) = node.as_call_or_write_node() {
            if write.read_name().as_slice() != b"[]" || write.call_operator_loc().is_some() {
                return;
            }
            let receiver = match write.receiver() {
                Some(receiver) => receiver,
                None => return,
            };
            check_receiver_gap_before_scanned_brackets(
                self,
                source,
                receiver.location().end_offset(),
                write.location().end_offset(),
                diagnostics,
            );
        }
    }
}

fn check_receiver_gap_before_brackets(
    cop: &dyn Cop,
    source: &SourceFile,
    receiver_end: usize,
    selector_start: Option<usize>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(selector_start) = selector_start else {
        return;
    };

    if receiver_end >= selector_start {
        return;
    }

    let bytes = source.as_bytes();
    let gap = &bytes[receiver_end..selector_start];
    if !is_bracket_gap(gap) {
        return;
    }

    let (line, col) = source.offset_to_line_col(receiver_end);
    diagnostics.push(cop.diagnostic(
        source,
        line,
        col,
        "Remove the space before the opening brackets.".to_string(),
    ));
}

fn is_bracket_gap(gap: &[u8]) -> bool {
    if gap.is_empty() {
        return false;
    }

    if gap.iter().all(|&byte| is_horizontal_space(byte)) {
        return true;
    }

    let Some(backslash_pos) = gap.iter().position(|&byte| byte == b'\\') else {
        return false;
    };

    if !gap[..backslash_pos]
        .iter()
        .all(|&byte| is_horizontal_space(byte))
    {
        return false;
    }

    let after_backslash = &gap[backslash_pos + 1..];
    let Some(after_newline) = after_backslash
        .strip_prefix(b"\r\n")
        .or_else(|| after_backslash.strip_prefix(b"\n"))
    else {
        return false;
    };

    after_newline.iter().all(|&byte| is_horizontal_space(byte))
}

fn is_horizontal_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

fn check_receiver_gap_before_scanned_brackets(
    cop: &dyn Cop,
    source: &SourceFile,
    receiver_end: usize,
    node_end: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let bytes = source.as_bytes();
    let selector_start = bytes[receiver_end..node_end]
        .iter()
        .position(|&byte| byte == b'[')
        .map(|offset| receiver_end + offset);
    check_receiver_gap_before_brackets(cop, source, receiver_end, selector_start, diagnostics);
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeBrackets, "cops/layout/space_before_brackets");

    #[test]
    fn index_operator_write_offense() {
        let source = b"value = nil\nvalue [0] += 1\n";
        let diagnostics = crate::testutil::run_cop_full(&SpaceBeforeBrackets, source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected one offense: {diagnostics:?}"
        );
    }

    #[test]
    fn continued_bracket_chain_offense() {
        let source = b"foo[1] \\\n  [0]\n";
        let diagnostics = crate::testutil::run_cop_full(&SpaceBeforeBrackets, source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected one offense for continued bracket access: {diagnostics:?}"
        );
    }

    #[test]
    fn continued_array_argument_no_offense() {
        let source = b"foo \\\n  [0]\n";
        let diagnostics = crate::testutil::run_cop_full(&SpaceBeforeBrackets, source);
        assert!(
            diagnostics.is_empty(),
            "Expected no offenses for continued array argument: {diagnostics:?}"
        );
    }
}
