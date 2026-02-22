use crate::cop::node_type::SUPER_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SuperWithArgsParentheses;

impl Cop for SuperWithArgsParentheses {
    fn name(&self) -> &'static str {
        "Style/SuperWithArgsParentheses"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[SUPER_NODE]
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
        let super_node = match node.as_super_node() {
            Some(s) => s,
            None => return,
        };

        // Must have arguments
        if super_node.arguments().is_none() {
            return;
        }

        // Check if parentheses are missing
        if super_node.lparen_loc().is_some() {
            return;
        }

        let loc = super_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use parentheses for `super` with arguments.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SuperWithArgsParentheses,
        "cops/style/super_with_args_parentheses"
    );
}
