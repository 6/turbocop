/// Lint/UriEscapeUnescape
///
/// Investigation findings:
/// - FN root cause: nitrocop only checked `escape` and `unescape`, but RuboCop also checks
///   `encode` and `decode`. Added all four methods.
/// - FP root cause: `constant_short_name()` returns just the final segment (e.g., `URI`) for qualified
///   paths like `SomeModule::URI`, causing false matches. RuboCop's NodePattern requires the
///   receiver to be `(const nil :URI)` (unqualified) or `(const cbase :URI)` (top-level `::URI`).
///   Fixed by checking that ConstantPathNode has no parent (for `::URI`) or that the receiver is
///   a ConstantReadNode (for bare `URI`), rejecting qualified paths like `SomeModule::URI`.
/// - Message format: updated to match RuboCop's exact format including replacement suggestions.
/// - Offense location: RuboCop reports the full send expression, not just the method name.
///   Updated to use the full call location.
// Handles both as_constant_read_node and as_constant_path_node (qualified constants like ::URI)
use crate::cop::shared::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UriEscapeUnescape;

/// Check if the receiver is exactly `URI` (unqualified) or `::URI` (top-level constant).
/// Rejects qualified paths like `SomeModule::URI`.
fn is_uri_constant(receiver: &ruby_prism::Node<'_>) -> Option<bool> {
    // Bare `URI` - ConstantReadNode with name "URI"
    if let Some(cr) = receiver.as_constant_read_node() {
        if cr.name().as_slice() == b"URI" {
            return Some(false); // not top-level (no ::)
        }
        return None;
    }
    // `::URI` - ConstantPathNode with no parent and name "URI"
    if let Some(cp) = receiver.as_constant_path_node() {
        // parent is None means `::URI` (cbase), parent is Some means `SomeModule::URI`
        if cp.parent().is_none() {
            if let Some(name) = cp.name() {
                if name.as_slice() == b"URI" {
                    return Some(true); // top-level (::)
                }
            }
        }
        return None;
    }
    None
}

impl Cop for UriEscapeUnescape {
    fn name(&self) -> &'static str {
        "Lint/UriEscapeUnescape"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        let is_escape_group = method_name == b"escape" || method_name == b"encode";
        let is_unescape_group = method_name == b"unescape" || method_name == b"decode";
        if !is_escape_group && !is_unescape_group {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_top_level = match is_uri_constant(&receiver) {
            Some(tl) => tl,
            None => return,
        };

        let double_colon = if is_top_level { "::" } else { "" };
        let method_str = std::str::from_utf8(method_name).unwrap_or("escape");

        let replacements = if is_escape_group {
            "`CGI.escape`, `URI.encode_www_form` or `URI.encode_www_form_component`"
        } else {
            "`CGI.unescape`, `URI.decode_www_form` or `URI.decode_www_form_component`"
        };

        let message = format!(
            "`{}URI.{}` method is obsolete and should not be used. Instead, use {} depending on your specific use case.",
            double_colon, method_str, replacements
        );

        // RuboCop reports the offense on the full send expression
        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UriEscapeUnescape, "cops/lint/uri_escape_unescape");
}
