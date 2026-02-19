use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::ARRAY_NODE;

pub struct FirstArrayElementLineBreak;

impl Cop for FirstArrayElementLineBreak {
    fn name(&self) -> &'static str {
        "Layout/FirstArrayElementLineBreak"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let _allow_implicit = config.get_bool("AllowImplicitArrayLiterals", false);
        let _allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        let array = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let opening = match array.opening_loc() {
            Some(loc) => loc,
            None => return, // Implicit array
        };
        let closing = match array.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        let elements: Vec<ruby_prism::Node<'_>> = array.elements().iter().collect();
        if elements.is_empty() {
            return;
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing.start_offset());

        // Only check multiline arrays
        if open_line == close_line {
            return;
        }

        // First element should be on a new line
        let first = &elements[0];
        let (first_line, first_col) = source.offset_to_line_col(first.location().start_offset());

        if first_line == open_line {
            diagnostics.push(self.diagnostic(
                source,
                first_line,
                first_col,
                "Add a line break before the first element of a multi-line array.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        FirstArrayElementLineBreak,
        "cops/layout/first_array_element_line_break"
    );
}
