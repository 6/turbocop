use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE};

pub struct RedundantRegexpArgument;

/// Methods where a regexp argument can be replaced with a string.
/// Must match vendor RuboCop's RESTRICT_ON_SEND list.
const TARGET_METHODS: &[&[u8]] = &[
    b"byteindex", b"byterindex",
    b"gsub", b"gsub!",
    b"partition", b"rpartition",
    b"scan",
    b"split",
    b"start_with?",
    b"sub", b"sub!",
];

impl Cop for RedundantRegexpArgument {
    fn name(&self) -> &'static str {
        "Style/RedundantRegexpArgument"
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
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        if !TARGET_METHODS.iter().any(|m| *m == name) {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument must be a simple regexp literal
        let regex = match arg_list[0].as_regular_expression_node() {
            Some(r) => r,
            None => return,
        };

        // Check if the regex is deterministic (no special regex chars)
        let content = regex.content_loc().as_slice();
        if !is_deterministic_regexp(content) {
            return;
        }

        // Skip single space regexps: / / is idiomatic
        if content == b" " {
            return;
        }

        // Check for flags by looking at the closing loc
        // If the regexp has flags like /foo/i, skip
        let closing = regex.closing_loc();
        let close_bytes = closing.as_slice();
        // Closing should just be "/" with no trailing flags
        if close_bytes.len() > 1 {
            return;
        }

        let loc = arg_list[0].location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use string `\"` instead of regexp `/` as the argument.".to_string(),
        ));
    }
}

fn is_deterministic_regexp(content: &[u8]) -> bool {
    // Check if regexp content is a simple literal string with no special chars
    for &b in content {
        match b {
            b'.' | b'*' | b'+' | b'?' | b'(' | b')' | b'[' | b']'
            | b'{' | b'}' | b'^' | b'$' | b'|' | b'\\' => return false,
            _ => {}
        }
    }
    !content.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantRegexpArgument, "cops/style/redundant_regexp_argument");
}
