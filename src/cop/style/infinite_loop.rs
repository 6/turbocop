use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{FALSE_NODE, TRUE_NODE, UNTIL_NODE, WHILE_NODE};

pub struct InfiniteLoop;

impl Cop for InfiniteLoop {
    fn name(&self) -> &'static str {
        "Style/InfiniteLoop"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[FALSE_NODE, TRUE_NODE, UNTIL_NODE, WHILE_NODE]
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
        // Check `while true`
        if let Some(while_node) = node.as_while_node() {
            let predicate = while_node.predicate();
            if predicate.as_true_node().is_some() {
                let kw_loc = while_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `Kernel#loop` for infinite loops.".to_string(),
                ));
            }
        }

        // Check `until false`
        if let Some(until_node) = node.as_until_node() {
            let predicate = until_node.predicate();
            if predicate.as_false_node().is_some() {
                let kw_loc = until_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `Kernel#loop` for infinite loops.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InfiniteLoop, "cops/style/infinite_loop");
}
