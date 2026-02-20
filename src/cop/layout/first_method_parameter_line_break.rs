use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::DEF_NODE;

pub struct FirstMethodParameterLineBreak;

impl Cop for FirstMethodParameterLineBreak {
    fn name(&self) -> &'static str {
        "Layout/FirstMethodParameterLineBreak"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let lparen_loc = match def_node.lparen_loc() {
            Some(loc) => loc,
            None => return,
        };
        let rparen_loc = match def_node.rparen_loc() {
            Some(loc) => loc,
            None => return,
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let (open_line, _) = source.offset_to_line_col(lparen_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(rparen_loc.start_offset());

        // Only check multiline parameter lists
        if open_line == close_line {
            return;
        }

        // Find the first parameter
        let requireds: Vec<ruby_prism::Node<'_>> = params.requireds().iter().collect();
        let optionals: Vec<ruby_prism::Node<'_>> =
            params.optionals().iter().collect();

        let first_offset = if !requireds.is_empty() {
            requireds[0].location().start_offset()
        } else if !optionals.is_empty() {
            optionals[0].location().start_offset()
        } else if let Some(rest) = params.rest() {
            rest.location().start_offset()
        } else {
            return;
        };

        let (first_line, first_col) = source.offset_to_line_col(first_offset);

        if first_line == open_line {
            diagnostics.push(self.diagnostic(
                source,
                first_line,
                first_col,
                "Add a line break before the first parameter of a multi-line method parameter definition.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        FirstMethodParameterLineBreak,
        "cops/layout/first_method_parameter_line_break"
    );
}
