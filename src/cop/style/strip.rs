use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects `lstrip.rstrip` and `rstrip.lstrip` and prefers `strip`.
///
/// ## Investigation notes
///
/// Historical corpus mismatches came from two contexts:
/// - implicit-receiver chains like `lstrip.rstrip`, which RuboCop still flags
/// - multiline receiver chains ending in `lstrip.rstrip`, where `node.location()`
///   starts at the beginning of the whole receiver chain instead of the first
///   strip selector
///
/// The cop now accepts missing inner receivers and reports/autocorrects from
/// the first strip selector (`inner_call.message_loc()`), matching RuboCop's
/// `first_send.loc.selector`.
pub struct Strip;

impl Cop for Strip {
    fn name(&self) -> &'static str {
        "Style/Strip"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let outer_name = call.name();
        let outer_bytes = outer_name.as_slice();

        // Must be lstrip or rstrip with no arguments
        if !matches!(outer_bytes, b"lstrip" | b"rstrip") {
            return;
        }
        if call.arguments().is_some() {
            return;
        }

        // Receiver must be a call to the opposite strip method
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        if let Some(inner_call) = receiver.as_call_node() {
            let inner_name = inner_call.name();
            let inner_bytes = inner_name.as_slice();

            // Must be the other strip method
            let is_pair = (outer_bytes == b"lstrip" && inner_bytes == b"rstrip")
                || (outer_bytes == b"rstrip" && inner_bytes == b"lstrip");

            if is_pair && inner_call.arguments().is_none() {
                // Get the full methods string for the message
                let inner_str = std::str::from_utf8(inner_bytes).unwrap_or("");
                let outer_str = std::str::from_utf8(outer_bytes).unwrap_or("");
                let methods = format!("{}.{}", inner_str, outer_str);

                let inner_loc = inner_call
                    .message_loc()
                    .unwrap_or_else(|| inner_call.location());
                let (line, column) = source.offset_to_line_col(inner_loc.start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `strip` instead of `{}`.", methods),
                );
                // Autocorrect only the strip pair so multiline chaining keeps its
                // existing call operator, whitespace, and indentation.
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: inner_loc.start_offset(),
                        end: node.location().end_offset(),
                        replacement: "strip".to_string(),
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
    crate::cop_fixture_tests!(Strip, "cops/style/strip");
    crate::cop_autocorrect_fixture_tests!(Strip, "cops/style/strip");
}
