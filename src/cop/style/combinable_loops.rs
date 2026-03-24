use crate::cop::node_type::{CALL_NODE, FOR_NODE, PROGRAM_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for consecutive loops over the same data that can be combined.
///
/// ## Investigation Notes
///
/// FP root cause: nitrocop included non-looping methods (map, flat_map, select,
/// reject, collect) in the loop method list. RuboCop only considers methods
/// starting with "each" or ending with "_each". Also, the blank-line gap check
/// was wrong — RuboCop doesn't care about blank lines between consecutive loops,
/// only about intervening *statements*. The `left_sibling` in RuboCop is the
/// previous AST sibling, regardless of whitespace.
///
/// FN root cause: `for` loops were not handled at all (only CallNode was checked).
/// Methods like `each_key`, `each_value`, `each_pair`, `each_with_object` were
/// missing from the method list because it was a hardcoded allowlist instead of
/// using the `starts_with("each") || ends_with("_each")` pattern from RuboCop.
/// Also, RuboCop requires both loops to have bodies (not empty blocks).
pub struct CombinableLoops;

impl Cop for CombinableLoops {
    fn name(&self) -> &'static str {
        "Style/CombinableLoops"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FOR_NODE, PROGRAM_NODE, STATEMENTS_NODE]
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
        let stmt_list: Vec<ruby_prism::Node<'_>> =
            if let Some(stmts_node) = node.as_statements_node() {
                stmts_node.body().iter().collect()
            } else if let Some(prog_node) = node.as_program_node() {
                prog_node.statements().body().iter().collect()
            } else {
                return;
            };

        for i in 1..stmt_list.len() {
            let prev = &stmt_list[i - 1];
            let curr = &stmt_list[i];

            if let (Some(prev_info), Some(curr_info)) =
                (get_loop_info(source, prev), get_loop_info(source, curr))
            {
                if prev_info.receiver == curr_info.receiver
                    && prev_info.method == curr_info.method
                    && prev_info.arguments == curr_info.arguments
                {
                    let loc = curr.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Combine this loop with the previous loop.".to_string(),
                    ));
                }
            }
        }
    }
}

struct LoopInfo {
    receiver: String,
    method: String,
    arguments: String,
}

fn is_collection_looping_method(method_name: &str) -> bool {
    method_name.starts_with("each") || method_name.ends_with("_each")
}

fn get_loop_info(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<LoopInfo> {
    // Handle for loops
    if let Some(for_node) = node.as_for_node() {
        let collection = for_node.collection();
        let receiver_text = source
            .try_byte_slice(
                collection.location().start_offset(),
                collection.location().end_offset(),
            )?
            .to_string();
        return Some(LoopInfo {
            receiver: receiver_text,
            method: "for".to_string(),
            arguments: String::new(),
        });
    }

    // Handle method call loops (each, each_with_index, etc.)
    let call = node.as_call_node()?;
    let method_name = std::str::from_utf8(call.name().as_slice()).ok()?;

    if !is_collection_looping_method(method_name) {
        return None;
    }

    // Must have a block
    let block = call.block()?;

    // Both loops must have bodies (not empty blocks)
    if let Some(block_node) = block.as_block_node() {
        block_node.body()?;
    }

    let receiver = call.receiver()?;
    let receiver_text = source
        .try_byte_slice(
            receiver.location().start_offset(),
            receiver.location().end_offset(),
        )?
        .to_string();

    // Capture method arguments (e.g., each_with_object([]) — the `([])` part)
    let arguments_text = if let Some(args) = call.arguments() {
        source
            .try_byte_slice(args.location().start_offset(), args.location().end_offset())
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };

    Some(LoopInfo {
        receiver: receiver_text,
        method: method_name.to_string(),
        arguments: arguments_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CombinableLoops, "cops/style/combinable_loops");
}
