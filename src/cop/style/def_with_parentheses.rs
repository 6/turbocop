use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DefWithParentheses;

impl Cop for DefWithParentheses {
    fn name(&self) -> &'static str {
        "Style/DefWithParentheses"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Check for empty parentheses — lparen_loc present but no parameters
        let lparen_loc = match def_node.lparen_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        // If there are parameters, this is not our concern
        if let Some(params) = def_node.parameters() {
            if !params.requireds().is_empty()
                || !params.optionals().is_empty()
                || params.rest().is_some()
                || !params.posts().is_empty()
                || !params.keywords().is_empty()
                || params.keyword_rest().is_some()
                || params.block().is_some()
            {
                return Vec::new();
            }
        }

        // For endless methods, check that there's a space before `=` after `()`
        // RuboCop does not flag `def foo()= do_something` (no space before =)
        if def_node.end_keyword_loc().is_none() {
            // Endless method — check for space after rparen
            if let Some(rparen_loc) = def_node.rparen_loc() {
                let rparen_end = rparen_loc.start_offset() + rparen_loc.as_slice().len();
                let src = source.as_bytes();
                if rparen_end < src.len() && src[rparen_end] == b'=' {
                    // No space before `=`, don't flag
                    return Vec::new();
                }
            }
        }

        let (line, column) = source.offset_to_line_col(lparen_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Omit the parentheses in defs when the method doesn't accept any arguments."
                .to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DefWithParentheses, "cops/style/def_with_parentheses");
}
