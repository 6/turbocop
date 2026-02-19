use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::PRE_EXECUTION_NODE;

pub struct BeginBlock;

impl Cop for BeginBlock {
    fn name(&self) -> &'static str {
        "Style/BeginBlock"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[PRE_EXECUTION_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let pre_exe = match node.as_pre_execution_node() {
            Some(n) => n,
            None => return,
        };

        let kw_loc = pre_exe.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid the use of `BEGIN` blocks.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BeginBlock, "cops/style/begin_block");
}
