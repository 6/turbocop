use crate::cop::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct StringChars;

impl Cop for StringChars {
    fn name(&self) -> &'static str {
        "Style/StringChars"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE]
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

        // Must be `split` method
        if call.name().as_slice() != b"split" {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        // Must have exactly one argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let arg = &arg_list[0];

        // Check for split('') or split("")
        let is_empty_string = arg
            .as_string_node()
            .is_some_and(|s| s.unescaped().is_empty());

        // Check for split(//)
        let is_empty_regexp = arg
            .as_regular_expression_node()
            .is_some_and(|r| r.unescaped().is_empty());

        if !is_empty_string && !is_empty_regexp {
            return;
        }

        // Build the offense message using the source range from selector to end
        let msg_loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

        let offense_src = std::str::from_utf8(
            &source.content[msg_loc.start_offset()..node.location().end_offset()],
        )
        .unwrap_or("split(...)");

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `chars` instead of `{}`.", offense_src),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringChars, "cops/style/string_chars");
}
