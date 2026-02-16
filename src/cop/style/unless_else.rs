use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct UnlessElse;

impl Cop for UnlessElse {
    fn name(&self) -> &'static str {
        "Style/UnlessElse"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let unless_node = match node.as_unless_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Must have an else clause
        if unless_node.else_clause().is_none() {
            return Vec::new();
        }

        let kw_loc = unless_node.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not use `unless` with `else`. Rewrite these with the positive case first."
                .to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnlessElse, "cops/style/unless_else");
}
