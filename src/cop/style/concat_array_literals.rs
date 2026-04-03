use crate::cop::shared::node_type::{ARRAY_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-30):
/// FN=2 came from receiverless `concat [major, minor, micro, qualifier]`
/// calls inside `Array` subclasses. RuboCop still flags receiverless `concat`
/// sends when every argument is an array literal, but this cop required an
/// explicit receiver and skipped them. Fix: allow receiverless `concat` and,
/// for that path, build the RuboCop-style `push(...)` message from the array
/// elements so the corpus examples match.
pub struct ConcatArrayLiterals;

fn receiverless_preferred_message(
    source: &SourceFile,
    call: ruby_prism::CallNode<'_>,
    arg_list: &[ruby_prism::Node<'_>],
) -> Option<String> {
    let current_start = call
        .message_loc()
        .map_or_else(|| call.location().start_offset(), |loc| loc.start_offset());
    let current = source.try_byte_slice(current_start, call.location().end_offset())?;

    let mut elements = Vec::new();
    for arg in arg_list {
        let array = arg.as_array_node()?;
        for element in array.elements().iter() {
            elements.push(
                source
                    .byte_slice(
                        element.location().start_offset(),
                        element.location().end_offset(),
                        "",
                    )
                    .to_string(),
            );
        }
    }

    Some(format!(
        "Use `push({})` instead of `{}`.",
        elements.join(", "),
        current
    ))
}

impl Cop for ConcatArrayLiterals {
    fn name(&self) -> &'static str {
        "Style/ConcatArrayLiterals"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE]
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

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name != "concat" {
            return;
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // All arguments must be array literals
        let all_arrays = arg_list.iter().all(|arg| arg.as_array_node().is_some());
        if !all_arrays {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let msg = if call.receiver().is_none() {
            receiverless_preferred_message(source, call, &arg_list).unwrap_or_else(|| {
                "Use `push` with elements as arguments instead of `concat` with array brackets."
                    .to_string()
            })
        } else {
            "Use `push` with elements as arguments instead of `concat` with array brackets."
                .to_string()
        };

        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConcatArrayLiterals, "cops/style/concat_array_literals");
}
