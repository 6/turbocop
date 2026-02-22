use crate::cop::node_type::STRING_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CharacterLiteral;

impl Cop for CharacterLiteral {
    fn name(&self) -> &'static str {
        "Style/CharacterLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
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
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Do not use the character literal - use string literal instead.".to_string(),
        );
        if let Some(ref mut corr) = corrections {
            // Replace ?x with "x" (or ?\n with "\n" etc.)
            let content = string_node.unescaped();
            let replacement = if content.len() == 1
                && content[0].is_ascii_graphic()
                && content[0] != b'\\'
                && content[0] != b'"'
            {
                format!("\"{}\"", content[0] as char)
            } else {
                // For escape sequences like ?\n, use the source text after ?
                let src = &node_source[1..];
                format!("\"{}\"", std::str::from_utf8(src).unwrap_or("?"))
            };
            corr.push(crate::correction::Correction {
                start: loc.start_offset(),
                end: loc.end_offset(),
                replacement,
                cop_name: self.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CharacterLiteral, "cops/style/character_literal");
    crate::cop_autocorrect_fixture_tests!(CharacterLiteral, "cops/style/character_literal");
}
