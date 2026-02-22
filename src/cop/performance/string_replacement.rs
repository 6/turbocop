use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct StringReplacement;

impl Cop for StringReplacement {
    fn name(&self) -> &'static str {
        "Performance/StringReplacement"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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

        if call.name().as_slice() != b"gsub" {
            return;
        }

        // Must have a receiver (str.gsub)
        if call.receiver().is_none() {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 2 {
            return;
        }

        let mut args_iter = args.iter();
        let first_node = match args_iter.next() {
            Some(a) => a,
            None => return,
        };
        let second_node = match args_iter.next() {
            Some(a) => a,
            None => return,
        };

        let first = match first_node.as_string_node() {
            Some(s) => s,
            None => return,
        };

        let second = match second_node.as_string_node() {
            Some(s) => s,
            None => return,
        };

        // Both must be single-character strings
        if first.unescaped().len() != 1 || second.unescaped().len() != 1 {
            return;
        }

        // RuboCop points at the gsub method name (node.loc.selector), not the whole expression
        let loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, "Use `tr` instead of `gsub` when replacing single characters.".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(StringReplacement, "cops/performance/string_replacement");
}
