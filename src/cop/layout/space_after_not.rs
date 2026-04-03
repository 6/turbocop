use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=918, FN=0.
///
/// FP=918 root cause: Prism represents both `!expr` and `not expr` as call nodes
/// with method name `"!"`. The previous implementation keyed only on call name and
/// receiver gap, so it flagged `not` keyword usage as if it were `! expr`.
///
/// Fix: require the operator token in source to be the literal `!` before checking
/// spacing. This preserves offenses for `! expr` and ignores `not expr`.
///
/// Remaining gap: none identified from this corpus slice; per-cop gate validates.
pub struct SpaceAfterNot;

impl Cop for SpaceAfterNot {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterNot"
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
        let bang_start = bang_loc.start_offset();
        let bang_end = bang_loc.end_offset();
        if source.as_bytes().get(bang_start..bang_end) != Some(b"!") {
            return;
        }
        let recv_start = call.receiver().unwrap().location().start_offset();
        if recv_start > bang_end {
            let between = &source.as_bytes()[bang_end..recv_start];
            if between.iter().any(|b| b.is_ascii_whitespace()) {
                let (line, column) = source.offset_to_line_col(bang_start);
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not leave space between `!` and its argument.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: bang_end,
                        end: recv_start,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAfterNot, "cops/layout/space_after_not");
    crate::cop_autocorrect_fixture_tests!(SpaceAfterNot, "cops/layout/space_after_not");
}
