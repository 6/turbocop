use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::POST_EXECUTION_NODE;

pub struct EndBlock;

impl Cop for EndBlock {
    fn name(&self) -> &'static str {
        "Style/EndBlock"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[POST_EXECUTION_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let post_exe = match node.as_post_execution_node() {
            Some(n) => n,
            None => return,
        };

        let kw_loc = post_exe.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid the use of `END` blocks. Use `Kernel#at_exit` instead.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EndBlock, "cops/style/end_block");
}
