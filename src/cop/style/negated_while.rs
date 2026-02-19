use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, WHILE_NODE};

pub struct NegatedWhile;

impl Cop for NegatedWhile {
    fn name(&self) -> &'static str {
        "Style/NegatedWhile"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, WHILE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let while_node = match node.as_while_node() {
            Some(n) => n,
            None => return,
        };

        let predicate = while_node.predicate();
        if let Some(call) = predicate.as_call_node() {
            if call.name().as_slice() == b"!" {
                let kw_loc = while_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, "Favor `until` over `while` for negative conditions.".to_string()));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(NegatedWhile, "cops/style/negated_while");
}
