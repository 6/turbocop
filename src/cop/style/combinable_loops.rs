use crate::cop::node_type::{CALL_NODE, PROGRAM_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CombinableLoops;

impl Cop for CombinableLoops {
    fn name(&self) -> &'static str {
        "Style/CombinableLoops"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, PROGRAM_NODE, STATEMENTS_NODE]
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
        // Check in class/module/method bodies for consecutive loops
        // Note: ProgramNode's StatementsNode is visited via visit_statements_node
        // directly (not through generic visit()), so visit_branch_node_enter is
        // NOT called for it. We handle ProgramNode explicitly here.
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
                // Check that loops are truly consecutive (no blank lines between them)
                let prev_end_line = source.offset_to_line_col(prev.location().end_offset()).0;
                let curr_start_line = source.offset_to_line_col(curr.location().start_offset()).0;
                if curr_start_line > prev_end_line + 1 {
                    continue; // There's a gap (blank line) between them
                }

                if prev_info.receiver == curr_info.receiver && prev_info.method == curr_info.method
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
}

fn get_loop_info(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<LoopInfo> {
    let call = node.as_call_node()?;
    let method_name = std::str::from_utf8(call.name().as_slice()).ok()?;

    // Only check looping methods
    if !matches!(
        method_name,
        "each"
            | "each_with_index"
            | "each_with_object"
            | "reverse_each"
            | "map"
            | "flat_map"
            | "select"
            | "reject"
            | "collect"
    ) {
        return None;
    }

    // Must have a block
    if call.block().is_none() {
        return None;
    }

    let receiver = call.receiver()?;
    let receiver_text = std::str::from_utf8(
        &source.as_bytes()[receiver.location().start_offset()..receiver.location().end_offset()],
    )
    .ok()?
    .to_string();

    Some(LoopInfo {
        receiver: receiver_text,
        method: method_name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CombinableLoops, "cops/style/combinable_loops");
}
