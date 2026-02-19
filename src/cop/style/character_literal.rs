use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::STRING_NODE;

pub struct CharacterLiteral;

impl Cop for CharacterLiteral {
    fn name(&self) -> &'static str {
        "Style/CharacterLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let string_node = match node.as_string_node() {
            Some(s) => s,
            None => return,
        };

        let opening = match string_node.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Character literals start with `?`
        if opening.as_slice() != b"?" {
            return;
        }

        // The total source of the node: ?x is 2 bytes, ?\n is 3 bytes
        // Allow meta and control characters like ?\C-\M-d (more than 3 bytes)
        let node_source = string_node.location().as_slice();
        if node_source.len() > 3 {
            return;
        }

        let loc = string_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not use the character literal - use string literal instead.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CharacterLiteral, "cops/style/character_literal");
}
