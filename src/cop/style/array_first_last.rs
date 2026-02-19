use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INTEGER_NODE};

pub struct ArrayFirstLast;

impl Cop for ArrayFirstLast {
    fn name(&self) -> &'static str {
        "Style/ArrayFirstLast"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE]
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

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name != "[]" {
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

        // Check if this is an assignment (arr[0] = 1), skip those
        // In Prism, []= is a separate method name
        // If the method name is [] and we're in a call, it's a read

        let arg = &arg_list[0];

        // Check for integer literal 0 or -1
        if let Some(int_node) = arg.as_integer_node() {
            let src = std::str::from_utf8(int_node.location().as_slice()).unwrap_or("");
            if let Ok(v) = src.parse::<i64>() {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());

                if v == 0 {
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `first`.".to_string(),
                    ));
                } else if v == -1 {
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use `last`.".to_string(),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArrayFirstLast, "cops/style/array_first_last");
}
