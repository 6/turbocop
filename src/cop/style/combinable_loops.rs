use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CombinableLoops;

impl Cop for CombinableLoops {
    fn name(&self) -> &'static str {
        "Style/CombinableLoops"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check in class/module/method bodies for consecutive loops
        let stmts = if let Some(stmts_node) = node.as_statements_node() {
            stmts_node
        } else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        let stmt_list: Vec<_> = stmts.body().iter().collect();

        for i in 1..stmt_list.len() {
            let prev = &stmt_list[i - 1];
            let curr = &stmt_list[i];

            if let (Some(prev_info), Some(curr_info)) = (
                get_loop_info(source, prev),
                get_loop_info(source, curr),
            ) {
                if prev_info.receiver == curr_info.receiver
                    && prev_info.method == curr_info.method
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

        diagnostics
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
        "each" | "each_with_index" | "each_with_object" | "reverse_each"
            | "map" | "flat_map" | "select" | "reject" | "collect"
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
