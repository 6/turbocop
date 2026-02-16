use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EndBlock;

impl Cop for EndBlock {
    fn name(&self) -> &'static str {
        "Style/EndBlock"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let post_exe = match node.as_post_execution_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let kw_loc = post_exe.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid the use of `END` blocks. Use `Kernel#at_exit` instead.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EndBlock, "cops/style/end_block");
}
