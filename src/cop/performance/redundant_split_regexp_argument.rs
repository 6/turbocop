use crate::cop::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantSplitRegexpArgument;

/// Check if regex content is a simple literal that could be replaced by a string.
/// Returns false for patterns with special regex characters like character classes,
/// quantifiers, alternation, anchors, etc.
fn is_simple_literal_regex(content: &[u8]) -> bool {
    if content.is_empty() {
        return false;
    }
    for &b in content {
        match b {
            // Any regex-special character means this is NOT a simple string
            b'.' | b'*' | b'+' | b'?' | b'|' | b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'^'
            | b'$' | b'\\' | b'#' => return false,
            _ => {}
        }
    }
    true
}

impl Cop for RedundantSplitRegexpArgument {
    fn name(&self) -> &'static str {
        "Performance/RedundantSplitRegexpArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, REGULAR_EXPRESSION_NODE]
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

        if call.name().as_slice() != b"split" {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.is_empty() {
            return;
        }

        // Check if first argument is a RegularExpressionNode with simple literal content
        let first_arg = match args.iter().next() {
            Some(a) => a,
            None => return,
        };
        let regex_node = match first_arg.as_regular_expression_node() {
            Some(r) => r,
            None => return,
        };

        let content = regex_node.content_loc().as_slice();
        if !is_simple_literal_regex(content) {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use string as argument instead of regexp.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantSplitRegexpArgument,
        "cops/performance/redundant_split_regexp_argument"
    );
}
