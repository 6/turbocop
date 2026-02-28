// Handles both as_constant_read_node and as_constant_path_node (qualified constants like ::URI)
use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UriDefaultParser;

impl Cop for UriDefaultParser {
    fn name(&self) -> &'static str {
        "Performance/UriDefaultParser"
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

        // Must be a call to `.new`
        if call.name().as_slice() != b"new" {
            return;
        }

        // The receiver must be `URI::Parser` or `::URI::Parser`
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let parser_path = match receiver.as_constant_path_node() {
            Some(cp) => cp,
            None => return,
        };

        // The name (rightmost part) must be `Parser`
        if parser_path.name().map(|n| n.as_slice()) != Some(b"Parser") {
            return;
        }

        // The parent must be `URI` (ConstantReadNode) or `::URI` (ConstantPathNode with no parent)
        let parent = match parser_path.parent() {
            Some(p) => p,
            None => return,
        };

        let double_colon;
        if let Some(cr) = parent.as_constant_read_node() {
            // Simple `URI::Parser.new`
            if cr.name().as_slice() != b"URI" {
                return;
            }
            double_colon = "";
        } else if let Some(cp) = parent.as_constant_path_node() {
            // `::URI::Parser.new` — parent is ConstantPathNode with no parent and name URI
            if cp.parent().is_some() {
                return;
            }
            if cp.name().map(|n| n.as_slice()) != Some(b"URI") {
                return;
            }
            double_colon = "::";
        } else {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `{double_colon}URI::DEFAULT_PARSER` instead of `{double_colon}URI::Parser.new`."
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(UriDefaultParser, "cops/performance/uri_default_parser");
}
